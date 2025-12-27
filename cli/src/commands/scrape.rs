use std::{
    collections::{HashMap, HashSet},
    path::Path,
    thread::sleep,
    time::Duration,
};

use arenabuddy_core::models::{Card, CardCollection};
use reqwest::StatusCode;
use tracingx::{debug, info};

use crate::{Error, Result, errors::ParseError};

const USER_AGENT: &str = "arenabuddy/1.0";

/// Execute the Scrape command
pub async fn execute(scryfall_host: &str, seventeen_lands_host: &str, output: &Path) -> Result<()> {
    info!("Scraping 17Lands data...");
    let seventeen_lands_data = scrape_seventeen_lands(seventeen_lands_host).await?;

    info!("Scraping Scryfall per-set data...");
    let scryfall_sets = scrape_sets(scryfall_host).await?;

    info!("Merging data from both sources...");
    let collection = merge(&seventeen_lands_data, &scryfall_sets);

    info!("Scraping completed successfully with {} cards", collection.cards.len());

    // Save the card collection to a binary protobuf file
    save_card_collection_to_file(&collection, output).await?;
    Ok(())
}

async fn scrape_sets(base_url: &str) -> Result<HashMap<String, HashMap<String, serde_json::Value>>> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;
    let mut ret: HashMap<String, HashMap<String, serde_json::Value>> = HashMap::new();

    let sets = find_sets(base_url, &client).await?;
    info!("Found {} sets", sets.len());

    for set in &sets {
        debug!("found set: {set}");
    }

    for set in sets {
        ret.insert(set.to_uppercase(), extract_set(base_url, &client, &set).await?);
    }

    for (set_name, set_cards) in &ret {
        info!("Set '{}' contains {} cards", set_name, set_cards.len());
    }
    Ok(ret)
}

async fn find_sets(base_url: &str, client: &reqwest::Client) -> Result<Vec<String>> {
    let response = client.get(format!("{base_url}/sets")).send().await?;
    response.error_for_status_ref()?;

    let data: serde_json::Value = response.json().await?;
    let mut sets = vec![];
    if let Some(data) = data["data"].as_array() {
        sets = data
            .iter()
            .filter_map(|set| {
                if set["set_type"]
                    .as_str()
                    .is_some_and(|st| st == "expansion" || st == "commander" || st == "alchemy")
                {
                    Some(set["code"].as_str().unwrap().to_owned())
                } else {
                    None
                }
            })
            .collect();
    }
    Ok(sets)
}

async fn extract_set(
    base_url: &str,
    client: &reqwest::Client,
    set: &str,
) -> Result<HashMap<String, serde_json::Value>, Error> {
    debug!("Extracting set: {set}");
    let set_query = format!("e:{set}");
    let query = vec![
        ("include_extras", "true"),
        ("include_variations", "true"),
        ("order", "set"),
        ("q", &set_query),
        ("unique", "prints"),
    ];
    let response = client
        .get(format!("{base_url}/cards/search"))
        .query(&query)
        .send()
        .await?;
    if response.status() == StatusCode::NOT_FOUND {
        debug!("Extracted {set} returned 404");
        return Ok(HashMap::new());
    }
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
    debug!("Extracted {} cards from {set}", ret.len());
    Ok(ret)
}

fn extract_set_cards(set_cards: &mut HashMap<String, serde_json::Value>, data: &serde_json::Value) {
    if let Some(data) = data["data"].as_array() {
        for card in data {
            if let Some(name) = card["name"].as_str() {
                set_cards.insert(name.to_owned(), card.clone());
            }
            if let Some(name) = card["printed_name"].as_str() {
                set_cards.insert(name.to_owned(), card.clone());
            }
            if let Some(faces) = card["card_faces"].as_array() {
                for face in faces {
                    if let Some(name) = face["name"].as_str() {
                        set_cards.insert(name.to_owned(), card.clone());
                    }
                    if let Some(name) = face["printed_name"].as_str() {
                        set_cards.insert(name.to_owned(), card.clone());
                    }
                }
            }
        }
    }
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

/// Merge 17Lands data with Scryfall per-set data
fn merge(
    seventeen_lands_cards: &Vec<HashMap<String, String>>,
    scryfall_sets: &HashMap<String, HashMap<String, serde_json::Value>>,
) -> CardCollection {
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
    let mut cards = vec![];
    let mut cards_by_id: HashSet<i64> = HashSet::new();

    // First pass: process 17Lands cards and match them with Scryfall data
    for card in seventeen_lands_cards {
        debug!("card: {card:?}");
        if let (Some(card_name), Some(card_id_str), Some(set)) =
            (card.get("name"), card.get("id"), card.get("expansion"))
        {
            let card_name = card_names_with_2_faces
                .get(card_name.split("//").next().unwrap_or("").trim())
                .unwrap_or(card_name);

            if let Ok(card_id) = card_id_str.parse::<i64>()
                && card_id != 0
                && !cards_by_id.contains(&card_id)
                && set != "ANA"
            {
                if let Some(card_data) = scryfall_sets
                    .get(set.as_str())
                    .and_then(|cards| cards.get(card_name.as_str()))
                {
                    let mut new_card = Card::from_json(card_data);
                    new_card.id = card_id;
                    cards.push(new_card);
                    cards_by_id.insert(card_id);
                    debug!(
                        "Found and added card from 17Lands [card_name='{}' arena_id={} set={}]",
                        card_name, card_id, set
                    );
                } else {
                    debug!("Card '{}' not found in Scryfall set '{}' data", card_name, set);
                }
            }
        }
    }

    let cards_from_seventeen_lands = cards.len();

    // Second pass: add any Scryfall cards with Arena IDs that weren't in 17Lands
    for (set_name, set_cards) in scryfall_sets {
        if set_name == "ANA" {
            continue;
        }
        for card_data in set_cards.values() {
            if let Some(arena_id) = card_data["arena_id"].as_i64()
                && arena_id != 0
                && !cards_by_id.contains(&arena_id)
            {
                let mut new_card = Card::from_json(card_data);
                new_card.id = arena_id;
                let card_name = new_card.name.clone();
                cards.push(new_card);
                cards_by_id.insert(arena_id);
                debug!(
                    "Found and added card from Scryfall only [card_name='{}' arena_id={} set={}]",
                    card_name, arena_id, set_name
                );
            }
        }
    }

    debug!(
        "Merged {} cards total ({} from 17Lands, {} Scryfall-only)",
        cards.len(),
        cards_from_seventeen_lands,
        cards.len() - cards_from_seventeen_lands
    );
    CardCollection { cards }
}
