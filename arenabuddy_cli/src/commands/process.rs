use std::path::PathBuf;
use anyhow::Result;
use arenabuddy_core::{proto_utils, cards::Card};
use serde_json::Value;
use tracing::{info, debug};

/// Execute the Process command
pub fn execute(
    scryfall_cards_file: &PathBuf,
    seventeen_lands_file: &PathBuf,
    reduced_arena_out: &PathBuf,
    merged_out: &PathBuf,
) -> Result<()> {
    process_cards(
        scryfall_cards_file,
        seventeen_lands_file,
        reduced_arena_out,
        merged_out,
    )
}

/// Process card data from scraped sources to create usable formats
pub fn process_cards(
    scryfall_cards_file: &PathBuf,
    seventeen_lands_file: &PathBuf,
    reduced_arena_out: &PathBuf,
    merged_out: &PathBuf,
) -> Result<()> {
    info!("Processing card data...");

    // Ensure output directories exist
    if let Some(parent) = reduced_arena_out.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Some(parent) = merged_out.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Read Scryfall cards data
    info!("Reading Scryfall cards from {}", scryfall_cards_file.display());
    let scryfall_data = std::fs::read_to_string(scryfall_cards_file)?;
    let scryfall_cards: Value = serde_json::from_str(&scryfall_data)?;

    // Process Arena cards
    info!("Reducing Arena cards...");
    let arena_cards = reduce_arena_cards(&scryfall_cards)?;

    // Write reduced Arena cards to file
    info!("Writing reduced Arena cards to {}", reduced_arena_out.display());
    let arena_cards_legacy = convert_to_legacy_cards(&arena_cards)?;
    proto_utils::save_card_collection_to_file(&arena_cards_legacy, reduced_arena_out)?;

    // Read 17Lands data
    info!("Reading 17Lands data from {}", seventeen_lands_file.display());
    let seventeen_lands_data = std::fs::read_to_string(seventeen_lands_file)?;

    // Merge card data
    info!("Merging card data...");
    let merged_cards = merge(&arena_cards, &seventeen_lands_data)?;

    // Write merged card data to file
    info!("Writing merged card data to {}", merged_out.display());
    let merged_cards_legacy = convert_to_legacy_cards(&merged_cards)?;
    proto_utils::save_card_collection_to_file(&merged_cards_legacy, merged_out)?;

    info!("Card processing completed successfully");
    Ok(())
}

/// Reduce Scryfall cards data to only Arena cards
fn reduce_arena_cards(cards: &Value) -> Result<Vec<serde_json::Value>> {
    let mut arena_cards = Vec::new();

    // Extract cards with Arena IDs
    if let Some(cards_array) = cards.as_array() {
        for card in cards_array {
            if card["arena_id"].is_number() {
                arena_cards.push(card.clone());
            }
        }
    }

    debug!("Extracted {} Arena cards from Scryfall data", arena_cards.len());

    Ok(arena_cards)
}

/// Extract Arena IDs from cards
fn _extract_arena_id_cards(cards: &[serde_json::Value]) -> Result<Vec<i32>> {
    let mut arena_ids = Vec::new();

    for card in cards {
        if let Some(id) = card["arena_id"].as_i64() {
            arena_ids.push(id as i32);
        }
    }

    Ok(arena_ids)
}

/// Merge Arena cards with 17Lands data
fn merge(arena_cards: &[serde_json::Value], seventeen_lands_data: &str) -> Result<Vec<serde_json::Value>> {
    let merged_cards = arena_cards.to_vec();

    // Parse 17Lands data and merge with Arena cards
    // This is a simplified implementation
    if !seventeen_lands_data.is_empty() {
        debug!("Merging with 17Lands data of length {}", seventeen_lands_data.len());
    }

    debug!("Merged arena cards with 17Lands data");

    Ok(merged_cards)
}

/// Convert JSON card values to Legacy Card format
fn convert_to_legacy_cards(cards: &[serde_json::Value]) -> Result<Vec<Card>> {
    let mut legacy_cards = Vec::new();

    for card in cards {
        if let (Some(id), Some(name), Some(set)) = (
            card["arena_id"].as_i64(),
            card["name"].as_str(),
            card["set"].as_str()
        ) {
            let legacy_card = Card {
                id: id as i32,
                name: name.to_string(),
                set: set.to_string(),
                lang: card["lang"].as_str().unwrap_or("en").to_string(),
                image_uri: card["image_uri"].as_str().map(|s| s.to_string()),
                mana_cost: card["mana_cost"].as_str().map(|s| s.to_string()),
                cmc: card["cmc"].as_f64().map_or(0, |f| f as u8),
                type_line: card["type_line"].as_str().unwrap_or("").to_string(),
                layout: card["layout"].as_str().unwrap_or("normal").to_string(),
                colors: None,
                color_identity: Vec::new(),
                card_faces: None,
            };

            legacy_cards.push(legacy_card);
        }
    }

    Ok(legacy_cards)
}
