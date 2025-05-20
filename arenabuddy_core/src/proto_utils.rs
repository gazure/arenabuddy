use std::path::Path;

use anyhow::Result;
use prost::Message;

use crate::{
    cards::{Card as LegacyCard, CardFace as LegacyCardFace},
    proto::{Card, CardCollection, CardFace},
};

/// Converts a legacy Card model to a protobuf Card
pub fn legacy_to_proto(legacy_card: &LegacyCard) -> Card {
    let mut proto_card = Card {
        id: i64::from(legacy_card.id),
        set: legacy_card.set.clone(),
        name: legacy_card.name.clone(),
        lang: legacy_card.lang.clone(),
        image_uri: legacy_card.image_uri.clone().unwrap_or_default(),
        mana_cost: legacy_card.mana_cost.clone().unwrap_or_default(),
        cmc: i32::from(legacy_card.cmc),
        type_line: legacy_card.type_line.clone(),
        layout: legacy_card.layout.clone(),
        colors: legacy_card.colors.clone().unwrap_or_default(),
        color_identity: legacy_card.color_identity.clone(),
        card_faces: Vec::new(),
    };

    // Convert card faces if they exist
    if let Some(legacy_faces) = &legacy_card.card_faces {
        proto_card.card_faces = legacy_faces.iter().map(legacy_face_to_proto).collect();
    }

    proto_card
}

/// Converts a legacy `CardFace` model to a protobuf `CardFace`
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

/// Converts a protobuf `CardFace` to a legacy `CardFace` model
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


/// Load a collection of cards from a binary protobuf file
pub fn load_card_collection_from_file(input_path: impl AsRef<Path>) -> Result<Vec<LegacyCard>> {
    // Read the file
    let bytes = std::fs::read(input_path)?;

    // Decode as CardCollection
    let collection = CardCollection::decode(bytes.as_slice())?;

    // Convert all protobuf cards to legacy cards
    let legacy_cards = collection.cards.iter().map(proto_to_legacy).collect();

    Ok(legacy_cards)
}
