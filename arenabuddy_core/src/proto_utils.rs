use anyhow::Result;
use prost::Message;
use std::path::Path;

use crate::{
    cards::{Card as LegacyCard, CardFace as LegacyCardFace},
    proto::{Card, CardCollection, CardFace},
};

/// Converts a legacy Card model to a protobuf Card
pub fn legacy_to_proto(legacy_card: &LegacyCard) -> Card {
    let mut proto_card = Card {
        id: legacy_card.id as i64,
        set: legacy_card.set.clone(),
        name: legacy_card.name.clone(),
        lang: legacy_card.lang.clone(),
        image_uri: legacy_card.image_uri.clone().unwrap_or_default(),
        mana_cost: legacy_card.mana_cost.clone().unwrap_or_default(),
        cmc: legacy_card.cmc as i32,
        type_line: legacy_card.type_line.clone(),
        layout: legacy_card.layout.clone(),
        colors: legacy_card
            .colors
            .clone()
            .unwrap_or_default(),
        color_identity: legacy_card.color_identity.clone(),
        card_faces: Vec::new(),
    };

    // Convert card faces if they exist
    if let Some(legacy_faces) = &legacy_card.card_faces {
        proto_card.card_faces = legacy_faces
            .iter()
            .map(legacy_face_to_proto)
            .collect();
    }

    proto_card
}

/// Converts a legacy CardFace model to a protobuf CardFace
fn legacy_face_to_proto(legacy_face: &LegacyCardFace) -> CardFace {
    CardFace {
        name: legacy_face.name.clone(),
        type_line: legacy_face.type_line.clone(),
        mana_cost: legacy_face.mana_cost.clone().unwrap_or_default(),
        image_uri: legacy_face.image_uri.clone(),
        colors: legacy_face.colors.clone().unwrap_or_default(),
    }
}

/// Converts a protobuf Card to a legacy Card model
pub fn proto_to_legacy(proto_card: &Card) -> LegacyCard {
    LegacyCard {
        id: proto_card.id as i32,
        set: proto_card.set.clone(),
        name: proto_card.name.clone(),
        lang: proto_card.lang.clone(),
        image_uri: if proto_card.image_uri.is_empty() {
            None
        } else {
            Some(proto_card.image_uri.clone())
        },
        mana_cost: if proto_card.mana_cost.is_empty() {
            None
        } else {
            Some(proto_card.mana_cost.clone())
        },
        cmc: proto_card.cmc as u8,
        type_line: proto_card.type_line.clone(),
        layout: proto_card.layout.clone(),
        colors: if proto_card.colors.is_empty() {
            None
        } else {
            Some(proto_card.colors.clone())
        },
        color_identity: proto_card.color_identity.clone(),
        card_faces: if proto_card.card_faces.is_empty() {
            None
        } else {
            Some(
                proto_card
                    .card_faces
                    .iter()
                    .map(proto_face_to_legacy)
                    .collect(),
            )
        },
    }
}

/// Converts a protobuf CardFace to a legacy CardFace model
fn proto_face_to_legacy(proto_face: &CardFace) -> LegacyCardFace {
    LegacyCardFace {
        name: proto_face.name.clone(),
        type_line: proto_face.type_line.clone(),
        mana_cost: if proto_face.mana_cost.is_empty() {
            None
        } else {
            Some(proto_face.mana_cost.clone())
        },
        image_uri: proto_face.image_uri.clone(),
        colors: if proto_face.colors.is_empty() {
            None
        } else {
            Some(proto_face.colors.clone())
        },
    }
}

/// Save a collection of cards to a binary protobuf file
pub fn save_card_collection_to_file(
    cards: &[LegacyCard],
    output_path: impl AsRef<Path>,
) -> Result<()> {
    let mut collection = CardCollection::new();
    
    // Convert all legacy cards to protobuf cards
    for card in cards {
        collection.add_card(legacy_to_proto(card));
    }
    
    // Encode the collection to bytes
    let encoded = collection.encode_to_vec();
    
    // Write to file
    std::fs::write(output_path, encoded)?;
    
    Ok(())
}

/// Load a collection of cards from a binary protobuf file
pub fn load_card_collection_from_file(
    input_path: impl AsRef<Path>,
) -> Result<Vec<LegacyCard>> {
    // Read the file
    let bytes = std::fs::read(input_path)?;
    
    // Decode as CardCollection
    let collection = CardCollection::decode(bytes.as_slice())?;
    
    // Convert all protobuf cards to legacy cards
    let legacy_cards = collection
        .cards
        .iter()
        .map(proto_to_legacy)
        .collect();
    
    Ok(legacy_cards)
}

/// Convert a JSON array of cards to a protobuf binary file
pub fn convert_json_to_proto_file(
    json_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
) -> Result<()> {
    // Read the JSON file
    let json_data = std::fs::read_to_string(json_path)?;
    
    // Parse the JSON as an array of legacy cards
    let legacy_cards: Vec<LegacyCard> = serde_json::from_str(&json_data)?;
    
    // Save to proto file
    save_card_collection_to_file(&legacy_cards, output_path)?;
    
    Ok(())
}