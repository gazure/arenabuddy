use anyhow::{Context, Result};
use ap_core::cards::{Card, CardFace};
use csv;
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Write};
use tracing::info;

const REDUCED_ARENA_OUT: &str = "reduced_arena.json";
const MERGED_OUT: &str = "merged.json";

fn reduce_arena_cards(card: &Value) -> Option<Card> {
    let mut reduced_card = Card {
        id: card["arena_id"]
            .as_i64()
            .map(|i| i32::try_from(i).unwrap_or(0))?,
        set: card["set"].as_str().map(|s| s.to_owned())?,
        name: card["name"].as_str().map(|s| s.to_owned())?,
        lang: card["lang"].as_str().map(|s| s.to_owned())?,
        image_uri: card["image_uris"]["normal"].as_str().map(|s| s.to_owned()),
        mana_cost: card["mana_cost"].as_str().map(|s| s.to_owned()),
        cmc: card["cmc"].as_f64().map(|f| f as f32)?,
        type_line: card["type_line"].as_str().map(|s| s.to_owned())?,
        layout: card["layout"].as_str().map(|s| s.to_owned())?,
        colors: card["colors"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        }),
        color_identity: card["color_identity"].as_array().map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })?,
        card_faces: None,
    };

    if let Some(card_faces) = card["card_faces"].as_array() {
        reduced_card.card_faces = Some(
            card_faces
                .iter()
                .filter_map(|face| {
                    Some(CardFace {
                        name: face["name"].as_str().map(|s| s.to_owned())?,
                        type_line: face["type_line"].as_str().map(|s| s.to_owned())?,
                        mana_cost: face["mana_cost"].as_str().map(|s| s.to_owned()),
                        image_uri: face["image_uris"]["normal"].as_str().map(|s| s.to_owned()),
                        colors: face["colors"].as_array().map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                                .collect()
                        }),
                    })
                })
                .collect(),
        );
    }

    Some(reduced_card)
}

fn extract_arena_id_cards(cards: &[Value]) -> Vec<Value> {
    cards
        .iter()
        .filter(|card| card["arena_id"].is_number())
        .cloned()
        .collect()
}

fn load_cards(cards_file: &str) -> Result<Vec<Value>> {
    let mut file =
        File::open(cards_file).with_context(|| format!("Failed to open file: {}", cards_file))?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)
        .with_context(|| format!("Failed to read file: {}", cards_file))?;
    contents = contents.replace("\\u2014", "-");
    let cards: Vec<Value> =
        serde_json::from_str(&contents).with_context(|| "Failed to parse JSON")?;
    Ok(cards)
}

fn load_seventeen_lands(cards_file: &str) -> Result<Vec<HashMap<String, String>>> {
    let mut reader = csv::Reader::from_path(cards_file)
        .with_context(|| format!("Failed to open CSV file: {}", cards_file))?;
    let records: Vec<HashMap<String, String>> = reader
        .deserialize()
        .collect::<std::result::Result<_, _>>()
        .with_context(|| "Failed to parse CSV records")?;
    Ok(records)
}

fn merge(
    seventeen_lands_cards: &[HashMap<String, String>],
    cards_by_name: &HashMap<String, Card>,
    cards_by_id: &mut HashMap<i32, Card>,
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
            let card_name = if let Some(full_name) = card_names_with_2_faces.get(
                &card_name
                    .split("//")
                    .next()
                    .unwrap_or("")
                    .trim()
                    .to_string(),
            ) {
                full_name
            } else {
                card_name
            };

            if let (Ok(card_id), Some(card_by_name)) = (
                card_id_str
                    .parse::<i32>()
                    .with_context(|| format!("Failed to parse card ID: {}", card_id_str)),
                cards_by_name.get(card_name),
            ) {
                if card_id != 0 && !cards_by_id.contains_key(&card_id) {
                    info!("Adding card {} with ID {}", card_name, card_id);
                    let mut new_card = card_by_name.clone();
                    new_card.id = card_id;
                    cards_by_id.insert(card_id, new_card);
                }
            }
        }
    }
}

pub async fn process(scryfall_cards_file: &str, seventeen_lands_file: &str) -> Result<()> {
    info!("Processing {}", scryfall_cards_file);
    let scryfall_cards = load_cards(scryfall_cards_file)
        .with_context(|| format!("Failed to load Scryfall cards from {}", scryfall_cards_file))?;

    let arena_cards = extract_arena_id_cards(&scryfall_cards);
    let reduced_arena_cards: Vec<Card> =
        arena_cards.iter().filter_map(reduce_arena_cards).collect();

    let cards_by_id: HashMap<i32, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.id, card.clone()))
        .collect();

    let cards_by_name: HashMap<String, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.name.clone(), card.clone()))
        .collect();

    info!("Found {} Arena ID cards", reduced_arena_cards.len());

    // Write reduced arena cards to file
    let reduced_json = serde_json::to_string(&reduced_arena_cards)
        .context("Failed to serialize reduced arena cards")?;
    let mut file = File::create(REDUCED_ARENA_OUT)
        .with_context(|| format!("Failed to create file: {}", REDUCED_ARENA_OUT))?;
    file.write_all(reduced_json.as_bytes())
        .with_context(|| format!("Failed to write to file: {}", REDUCED_ARENA_OUT))?;

    info!("Processing {}", seventeen_lands_file);
    let seventeen_lands_cards = load_seventeen_lands(seventeen_lands_file)
        .with_context(|| format!("Failed to load 17Lands cards from {}", seventeen_lands_file))?;
    info!("Found {} 17Lands cards", seventeen_lands_cards.len());

    let mut cards_by_id_mut = cards_by_id.clone();
    let merged = merge(
        &seventeen_lands_cards,
        &cards_by_name,
        &mut cards_by_id_mut,
        &reduced_arena_cards,
    );

    // Write merged cards to file
    let merged_json = serde_json::to_string(&merged).context("Failed to serialize merged cards")?;
    let mut file = File::create(MERGED_OUT)
        .with_context(|| format!("Failed to create file: {}", MERGED_OUT))?;
    file.write_all(merged_json.as_bytes())
        .with_context(|| format!("Failed to write to file: {}", MERGED_OUT))?;

    info!("Done");
    Ok(())
}
