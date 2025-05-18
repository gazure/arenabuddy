use std::collections::HashMap;

use anyhow::{Context, Result};
use arenabuddy_core::proto::{Card, CardFace, CardCollection};
use prost::Message;
use serde_json::Value;
use tokio::{fs::File, io::AsyncWriteExt};
use tracing::{debug, info};

const DEFAULT_SCRYFALL_CARDS: &str = crate::scrape::SCRYFALL_OUT;
const DEFAULT_SEVENTEEN_LANDS: &str = crate::scrape::SEVENTEEN_LANDS_OUT;

pub(crate) const REDUCED_ARENA_OUT: &str = "scrape_data/reduced_arena.pb";
pub(crate) const MERGED_OUT: &str = "src-tauri/data/cards-full.pb";

fn reduce_arena_cards(card: &Value) -> Option<Card> {
    let id = card["arena_id"].as_i64()?;
    
    let mut proto_card = Card {
        id,
        set: card["set"].as_str()?.to_owned(),
        name: card["name"].as_str()?.to_owned(),
        lang: card["lang"].as_str()?.to_owned(),
        image_uri: card["image_uris"]["normal"].as_str().unwrap_or_default().to_owned(),
        mana_cost: card["mana_cost"].as_str().unwrap_or_default().to_owned(),
        cmc: card["cmc"].as_f64().map_or(0, |f| f as i32),
        type_line: card["type_line"].as_str()?.to_owned(),
        layout: card["layout"].as_str()?.to_owned(),
        colors: Vec::new(),
        color_identity: Vec::new(),
        card_faces: Vec::new(),
    };
    
    // Parse array fields
    if let Some(colors) = card["colors"].as_array() {
        proto_card.colors = colors
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect();
    }
    
    if let Some(color_identity) = card["color_identity"].as_array() {
        proto_card.color_identity = color_identity
            .iter()
            .filter_map(|v| v.as_str().map(ToString::to_string))
            .collect();
    }
    
    // Parse card faces if present
    if let Some(faces) = card["card_faces"].as_array() {
        proto_card.card_faces = faces
            .iter()
            .filter_map(|face| {
                if !face.is_object() {
                    return None;
                }
                
                let mut card_face = CardFace {
                    name: face["name"].as_str()?.to_string(),
                    type_line: face["type_line"].as_str()?.to_string(),
                    mana_cost: face["mana_cost"].as_str().unwrap_or_default().to_string(),
                    image_uri: None,
                    colors: Vec::new(),
                };
                
                // Optional fields
                if let Some(image) = face.get("image_uris")
                    .and_then(|uris| uris.get("normal"))
                    .and_then(|uri| uri.as_str()) {
                    card_face.image_uri = Some(image.to_string());
                }
                
                if let Some(face_colors) = face["colors"].as_array() {
                    card_face.colors = face_colors
                        .iter()
                        .filter_map(|c| c.as_str().map(ToString::to_string))
                        .collect();
                }
                
                Some(card_face)
            })
            .collect();
    }
    
    Some(proto_card)
}

fn extract_arena_id_cards(cards: &[Value]) -> Vec<Value> {
    cards
        .iter()
        .filter(|card| card["arena_id"].is_number())
        .cloned()
        .collect()
}

async fn load_cards(cards_file: &str) -> Result<Vec<Value>> {
    let mut contents = tokio::fs::read_to_string(cards_file)
        .await
        .with_context(|| format!("Failed to read file: {cards_file}"))?;
    contents = contents.replace("\\u2014", "-");
    let cards: Vec<Value> =
        serde_json::from_str(&contents).with_context(|| "Failed to parse JSON")?;
    Ok(cards)
}

fn load_seventeen_lands(cards_file: &str) -> Result<Vec<HashMap<String, String>>> {
    let mut reader = csv::Reader::from_path(cards_file)
        .with_context(|| format!("Failed to open CSV file: {cards_file}"))?;
    let records: Vec<HashMap<String, String>> = reader
        .deserialize()
        .collect::<std::result::Result<_, _>>()
        .with_context(|| "Failed to parse CSV records")?;
    Ok(records)
}

fn merge(
    seventeen_lands_cards: &[HashMap<String, String>],
    cards_by_name: &HashMap<String, Card>,
    cards_by_id: &mut HashMap<i64, Card>,
    _arena_cards: &[Card],
) {
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
                    debug!("Adding card {} with ID {}", card_name, card_id);
                    let mut new_card = card_by_name.clone();
                    new_card.id = card_id;
                    cards_by_id.insert(card_id, new_card);
                }
            }
        }
    }
}

/// # Errors
///
/// Will return any underlying fs/io errors encountered while processing
pub async fn process(
    scryfall_cards_file: Option<&str>,
    seventeen_lands_file: Option<&str>,
) -> Result<()> {
    let scryfall_cards_file = scryfall_cards_file.unwrap_or(DEFAULT_SCRYFALL_CARDS);
    let seventeen_lands_file = seventeen_lands_file.unwrap_or(DEFAULT_SEVENTEEN_LANDS);

    info!("Processing {}", scryfall_cards_file);
    let scryfall_cards = load_cards(scryfall_cards_file)
        .await
        .with_context(|| format!("Failed to load Scryfall cards from {scryfall_cards_file}"))?;

    info!("loaded scryfall cards");

    let arena_cards = extract_arena_id_cards(&scryfall_cards);
    let reduced_arena_cards: Vec<Card> =
        arena_cards.iter().filter_map(reduce_arena_cards).collect();

    let mut cards_by_id: HashMap<i64, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.id, card.clone()))
        .collect();

    let cards_by_name: HashMap<String, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.name.clone(), card.clone()))
        .collect();

    info!("Found {} Arena ID cards", reduced_arena_cards.len());

    // Write reduced arena cards to file as protocol buffer
    let mut card_collection = CardCollection::new();
    let reduced_arena_cards_clone = reduced_arena_cards.clone(); // Clone for later use
    for card in reduced_arena_cards {
        card_collection.cards.push(card);
    }
    let mut buf = Vec::new();
    card_collection.encode(&mut buf)
        .context("Failed to serialize reduced arena cards to protobuf")?;
    
    let mut file = tokio::fs::File::create(REDUCED_ARENA_OUT)
        .await
        .with_context(|| format!("Failed to create file: {REDUCED_ARENA_OUT}"))?;
    file.write_all(&buf)
        .await
        .with_context(|| format!("Failed to write to file: {REDUCED_ARENA_OUT}"))?;

    info!("Processing {}", seventeen_lands_file);
    let seventeen_lands_cards = load_seventeen_lands(seventeen_lands_file)
        .with_context(|| format!("Failed to load 17Lands cards from {seventeen_lands_file}"))?;
    info!("Found {} 17Lands cards", seventeen_lands_cards.len());

    merge(
        &seventeen_lands_cards,
        &cards_by_name,
        &mut cards_by_id,
        &reduced_arena_cards_clone,
    );

    // Write merged cards to file as protocol buffer
    let mut merged_collection = CardCollection::new();
    for (_, card) in cards_by_id {
        merged_collection.cards.push(card);
    }
    let mut buf = Vec::new();
    merged_collection.encode(&mut buf)
        .context("Failed to serialize merged cards to protobuf")?;
    
    let mut file = File::create(MERGED_OUT)
        .await
        .with_context(|| format!("Failed to create file: {MERGED_OUT}"))?;
    file.write_all(&buf)
        .await
        .with_context(|| format!("Failed to write to file: {MERGED_OUT}"))?;

    info!("Done");
    Ok(())
}
