use std::{collections::HashMap, path::Path};

use anyhow::{Context, Result};
use arenabuddy_core::models::{Card, CardCollection};
use tracing::{debug, error, info};

/// Execute the Scrape command
pub async fn execute(
    scryfall_host: &str,
    seventeen_lands_host: &str,
    output_dir: &Path,
) -> Result<()> {
    // Scrape data from both sources

    info!("Scraping 17Lands data...");
    let seventeen_lands_data = scrape_seventeen_lands(seventeen_lands_host).await?;

    info!("Scraping Scryfall data...");
    let scryfall_data = scrape_scryfall(scryfall_host).await?;

    // Extract cards with Arena IDs
    let Some(cards_array) = scryfall_data.as_array() else {
        anyhow::bail!("Could not find cards array");
    };

    let cards: Vec<Card> = cards_array
        .iter()
        .filter(|c| c["arena_id"].is_number())
        .map(Card::from_json)
        .collect();

    debug!("Filtered to {} cards with Arena IDs", cards_array.len());

    let collection = CardCollection {
        cards: merge(cards, &seventeen_lands_data)?,
    };

    info!("Scraping completed successfully");

    // Save the card collection to a binary protobuf file
    save_card_collection_to_file(&collection, output_dir.join("cards.pb")).await?;
    save_to_s3(&collection).await?;
    Ok(())
}

/// Scrape card data from Scryfall API
async fn scrape_scryfall(base_url: &str) -> Result<serde_json::Value> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;

    // Get bulk data endpoint
    let response = client.get(format!("{base_url}/bulk-data")).send().await?;

    info!("Response: {}", response.status());
    response.error_for_status_ref()?;

    let data: serde_json::Value = response.json().await?;

    // Find and download all_cards data
    let Some(bulk_data) = data.get("data").and_then(|d| d.as_array()) else {
        anyhow::bail!("Could not find all_cards data")
    };
    for item in bulk_data {
        if item["type"] == "all_cards" {
            if let Some(download_uri) = item["download_uri"].as_str() {
                info!("Downloading {}", download_uri);

                let cards_response = client.get(download_uri).send().await?;
                cards_response.error_for_status_ref()?;

                let response_text = cards_response.text().await?;

                // Parse the saved text as JSON for the return value
                return Ok(serde_json::from_str(&response_text)?);
            }
        }
    }
    anyhow::bail!("No bulk cards found")
}

/// Scrape card data from 17Lands
async fn scrape_seventeen_lands(base_url: &str) -> Result<Vec<HashMap<String, String>>> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    let value = response.text().await?;

    let mut reader = csv::Reader::from_reader(value.as_bytes());
    reader
        .deserialize()
        .collect::<std::result::Result<_, _>>()
        .with_context(|| "Failed to parse CSV records")
}

/// Save a collection of cards to a binary protobuf file
pub async fn save_card_collection_to_file(
    cards: &CardCollection,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let data = cards.encode_to_vec();
    tokio::fs::write(output_path.as_ref(), &data).await?;
    Ok(())
}

/// Merge Arena cards with 17Lands data
fn merge(
    mut arena_cards: Vec<Card>,
    seventeen_lands_cards: &Vec<HashMap<String, String>>,
) -> Result<Vec<Card>> {
    let cards_by_name: HashMap<String, &Card> =
        arena_cards.iter().map(|c| (c.name.clone(), c)).collect();

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
        if let (Some(card_name), Some(card_id_str)) = (card.get("name"), card.get("id")) {
            let card_name = card_names_with_2_faces
                .get(card_name.split("//").next().unwrap_or("").trim())
                .unwrap_or(card_name);

            if let (Ok(card_id), Some(card_by_name)) = (
                card_id_str
                    .parse::<i64>()
                    .with_context(|| format!("Failed to parse card ID: {card_id_str}")),
                cards_by_name.get(card_name),
            ) {
                if card_id != 0 && !cards_by_id.contains_key(&card_id) {
                    let mut new_card = (*card_by_name).clone();
                    new_card.id = card_id;
                    new_cards.push(new_card);
                }
            }
        }
    }

    arena_cards.extend(new_cards);
    debug!("Merged arena cards with 17Lands data");

    Ok(arena_cards)
}

async fn save_to_s3(cards: &CardCollection) -> Result<()> {
    let encoded = cards.encode_to_vec();

    // Write to S3
    let s3_client = aws_sdk_s3::Client::new(&aws_config::load_from_env().await);
    let bucket_name = "arenabuddy-data";
    let key = "cards.pb";

    info!("uploading cards.pb to S3");
    let res = s3_client
        .put_object()
        .bucket(bucket_name)
        .key(key)
        .body(encoded.into())
        .send()
        .await;

    if let Err(err) = res {
        error!("Failed to upload cards.pb to S3: {}", err);
        return Err(err.into());
    }

    Ok(())
}
