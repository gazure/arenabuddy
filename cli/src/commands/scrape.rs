use std::{collections::HashMap, path::Path, time::Duration};

use arenabuddy_core::models::{Card, CardCollection};
use reqwest::Url;
use tracingx::{debug, info, warn};

use crate::{Error, Result, errors::ParseError};

/// Execute the Scrape command
pub async fn execute(scryfall_host: &str, seventeen_lands_host: &str, output: &Path) -> Result<()> {
    // Scrape data from both sources

    info!("Scraping 17Lands data...");
    let seventeen_lands_data = scrape_seventeen_lands(seventeen_lands_host).await?;

    info!("Scraping Scryfall data...");
    let scryfall_data = scrape_scryfall(scryfall_host).await?;

    // Extract cards with Arena IDs
    let Some(cards_array) = scryfall_data.as_array() else {
        return Err(Error::Invalid("Could not find cards array in scryfall data".to_owned()));
    };

    let cards: Vec<Card> = cards_array
        .iter()
        .filter(|c| c["arena_id"].is_number())
        .map(Card::from_json)
        .collect();

    debug!("Filtered to {} cards with Arena IDs", cards_array.len());

    let collection = CardCollection {
        cards: merge(cards, &seventeen_lands_data, scryfall_host).await?,
    };

    info!("Scraping completed successfully");

    // Save the card collection to a binary protobuf file
    save_card_collection_to_file(&collection, output).await?;
    Ok(())
}

/// Scrape card data from Scryfall API
async fn scrape_scryfall(base_url: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::builder().user_agent("arenabuddy/1.0").build()?;

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
    Err(Error::Invalid("No bulk cards found".to_owned()))
}

/// Scrape card data from 17Lands
async fn scrape_seventeen_lands(base_url: &str) -> Result<Vec<HashMap<String, String>>> {
    let client = reqwest::Client::builder().user_agent("arenabuddy/1.0").build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    let value = response.text().await?;

    let mut reader = csv::Reader::from_reader(value.as_bytes());
    reader
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

/// Search for a card by name using Scryfall API with basic rate limiting
async fn search_card_by_name(base_url: &str, card_name: &str) -> Result<Option<Card>> {
    let client = reqwest::Client::builder().user_agent("arenabuddy/1.0").build()?;

    tokio::time::sleep(Duration::from_millis(150)).await;

    let url = Url::parse_with_params(&format!("{base_url}/cards/search"), &[("q", card_name)])
        .map_err(|e| Error::Url(e.to_string()))?;

    debug!("Searching for card: {} at: {}", card_name, url);

    let response = client.get(url).send().await?;

    if response.status() == 404 {
        warn!("Card not found: {}", card_name);
        return Ok(None);
    }

    response.error_for_status_ref()?;
    let data: serde_json::Value = response.json().await?;

    // Get the first card from search results
    if let Some(cards_array) = data.get("data").and_then(|d| d.as_array())
        && let Some(first_card) = cards_array.first()
    {
        return Ok(Some(Card::from_json(first_card)));
    }

    Ok(None)
}

/// Merge Arena cards with 17Lands data
async fn merge(
    mut arena_cards: Vec<Card>,
    seventeen_lands_cards: &Vec<HashMap<String, String>>,
    scryfall_host: &str,
) -> Result<Vec<Card>> {
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
                } else if ["FIN", "EOE", "OM1"].contains(&set.as_str()) {
                    // Card not found in existing data, search Scryfall
                    info!("Card '{}' not found in Scryfall data, searching...", card_name);
                    if let Ok(Some(found_card)) = search_card_by_name(scryfall_host, card_name).await {
                        let mut new_card = found_card;
                        new_card.id = card_id;
                        new_cards.push(new_card);
                        debug!("Found and added card '{}' with arena_id {}", card_name, card_id);
                    } else {
                        warn!("Could not find card '{}' via Scryfall search", card_name);
                    }
                } else {
                    debug!("Opting to not search for {}", card_name);
                }
            }
        }
    }

    arena_cards.extend(new_cards);
    debug!("Merged arena cards with 17Lands data");

    Ok(arena_cards)
}
