use std::{collections::HashMap, path::Path, thread::sleep, time::Duration};

use arenabuddy_core::models::{Card, CardCollection};
use tracingx::{debug, info};

use crate::{Error, Result, errors::ParseError};

const EXTRA_SETS: [&str; 4] = ["FIN", "EOE", "OM1", "TLA"];
const USER_AGENT: &str = "arenabuddy/1.0";

/// Execute the Scrape command
pub async fn execute(scryfall_host: &str, seventeen_lands_host: &str, output: &Path) -> Result<()> {
    // Scrape data from both sources

    info!("Scraping 17Lands data...");
    let seventeen_lands_data = scrape_seventeen_lands(seventeen_lands_host).await?;

    info!("Scraping Scryfall data...");
    let scryfall_data = scrape_scryfall(scryfall_host).await?;

    // Extract cards with Arena IDs
    let Some(cards_array) = scryfall_data.as_array() else {
        return Err(Error::Invalid("Could not find cards array in scryfall data".into()));
    };

    let cards: Vec<Card> = cards_array
        .iter()
        .filter(|c| c["arena_id"].is_number())
        .map(Card::from_json)
        .collect();

    let extra_sets = scrape_sets(scryfall_host, EXTRA_SETS.as_slice()).await?;

    debug!("Filtered to {} cards with Arena IDs", cards_array.len());

    let collection = merge(cards, &seventeen_lands_data, &extra_sets);

    info!("Scraping completed successfully");

    // Save the card collection to a binary protobuf file
    save_card_collection_to_file(&collection, output).await?;
    Ok(())
}

async fn scrape_sets(
    base_url: &str,
    extra_sets: &[&str],
) -> Result<HashMap<String, HashMap<String, serde_json::Value>>> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let mut ret: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

    for set in extra_sets.iter().copied() {
        ret.insert(set.to_owned(), extract_set(base_url, &client, set).await?);
    }
    for (set_name, set_cards) in &ret {
        info!("Set '{}' contains {} cards", set_name, set_cards.len());
    }
    Ok(ret)
}

async fn extract_set(
    base_url: &str,
    client: &reqwest::Client,
    set: &str,
) -> Result<HashMap<String, serde_json::Value>, Error> {
    let set_q = format!("e:{set}");
    let query = vec![
        ("include_extras", "true"),
        ("include_variations", "true"),
        ("order", "set"),
        ("q", &set_q),
        ("unique", "prints"),
    ];
    let response = client
        .get(format!("{base_url}/cards/search"))
        .query(&query)
        .send()
        .await?;
    response.error_for_status_ref()?;
    let mut data: serde_json::Value = response.json().await?;
    let mut ret = HashMap::new();
    extract_set_cards(&mut ret, &data);
    while let Some(next_page) = data["next_page"].as_str() {
        let response = client.get(next_page).send().await?;
        response.error_for_status_ref()?;

        data = response.json().await?;
        extract_set_cards(&mut ret, &data);
        sleep(Duration::from_millis(150));
    }
    Ok(ret)
}

fn extract_set_cards(set_cards: &mut HashMap<String, serde_json::Value>, data: &serde_json::Value) {
    if let Some(data) = data["data"].as_array() {
        for card in data {
            if let Some(name) = card["name"].as_str() {
                debug!("Inserting: {name}");
                set_cards.insert(name.to_owned(), card.clone());
            }
            if let Some(name) = card["printed_name"].as_str() {
                debug!("Inserting: {name}");
                set_cards.insert(name.to_owned(), card.clone());
            }
            if let Some(faces) = card["card_faces"].as_array() {
                for face in faces {
                    if let Some(name) = face["name"].as_str() {
                        debug!("Inserting: {name}");
                        set_cards.insert(name.to_owned(), card.clone());
                    }
                    if let Some(name) = face["printed_name"].as_str() {
                        debug!("Inserting: {name}");
                        set_cards.insert(name.to_owned(), card.clone());
                    }
                }
            }
        }
    }
}

/// Scrape card data from Scryfall API
async fn scrape_scryfall(base_url: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

    // Get bulk data endpoint
    let response = client.get(format!("{base_url}/bulk-data")).send().await?;

    info!("Response: {}", response.status());
    response.error_for_status_ref()?;

    let data: serde_json::Value = response.json().await?;

    // Find and download all_cards data
    let Some(bulk_data) = data.get("data").and_then(|d| d.as_array()) else {
        return Err(Error::Invalid("Could not find all_cards data from scryfall".to_owned()));
    };
    for item in bulk_data {
        if item["type"] == "all_cards"
            && let Some(download_uri) = item["download_uri"].as_str()
        {
            info!("Downloading {}", download_uri);

            let cards_response = client.get(download_uri).send().await?;
            cards_response.error_for_status_ref()?;

            let response_text = cards_response.text().await?;

            // Parse the saved text as JSON for the return value
            return Ok(serde_json::from_str(&response_text).map_err(ParseError::from)?);
        }
    }
    Err(Error::Invalid("No bulk cards found".into()))
}

/// Scrape card data from 17Lands
async fn scrape_seventeen_lands(base_url: &str) -> Result<Vec<HashMap<String, String>>> {
    let client = reqwest::Client::builder().user_agent("arenabuddy/1.0").build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    csv::Reader::from_reader(response.bytes().await?.as_ref())
        .deserialize()
        .map(|result| result.map_err(ParseError::from).map_err(Error::from))
        .collect()
}

/// Save a collection of cards to a binary protobuf file
async fn save_card_collection_to_file(cards: &CardCollection, output_path: impl AsRef<Path>) -> Result<()> {
    let data = cards.encode_to_vec();
    tokio::fs::write(output_path.as_ref(), &data).await?;
    Ok(())
}

/// Merge Arena cards with 17Lands data
fn merge(
    mut arena_cards: Vec<Card>,
    seventeen_lands_cards: &Vec<HashMap<String, String>>,
    extra_sets: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> CardCollection {
    let cards_by_name: HashMap<String, &Card> = arena_cards.iter().map(|c| (c.name.clone(), c)).collect();

    let cards_by_id: HashMap<i64, &Card> = arena_cards.iter().map(|c| (c.id, c)).collect();
    // Create map of two-faced cards
    let card_names_with_2_faces: HashMap<String, String> = seventeen_lands_cards
        .iter()
        .filter_map(|card| {
            let name = card.get("name")?;
            if name.contains("//") {
                Some((name.split("//").next()?.trim().to_string(), name.clone()))
            } else {
                None
            }
        })
        .collect();
    let mut new_cards = vec![];

    for card in seventeen_lands_cards {
        if let (Some(card_name), Some(card_id_str), Some(set)) =
            (card.get("name"), card.get("id"), card.get("expansion"))
        {
            let card_name = card_names_with_2_faces
                .get(card_name.split("//").next().unwrap_or("").trim())
                .unwrap_or(card_name);

            if let Ok(card_id) = card_id_str.parse::<i64>()
                && card_id != 0
                && !cards_by_id.contains_key(&card_id)
                && set != "ANA"
            {
                if let Some(card_by_name) = cards_by_name.get(card_name) {
                    // Found existing card by name, create new card with 17Lands arena_id
                    let mut new_card = (*card_by_name).clone();
                    new_card.id = card_id;
                    new_cards.push(new_card);
                } else if EXTRA_SETS.contains(&set.as_str()) {
                    info!("Card '{}' not found in Scryfall data, searching...", card_name);

                    if let Some(card_data) = extra_sets
                        .get(set.as_str())
                        .and_then(|cards| cards.get(card_name.as_str()))
                    {
                        let mut new_card = Card::from_json(card_data);
                        new_card.id = card_id;
                        new_cards.push(new_card);
                        debug!(
                            "Found and added card in extra set data [card_name='{}' arena_id={}]",
                            card_name, card_id
                        );
                    }
                } else {
                    debug!("Opting to not search for {}", card_name);
                }
            }
        }
    }

    arena_cards.extend(new_cards);
    debug!("Merged arena cards with 17Lands data");
    CardCollection { cards: arena_cards }
}
