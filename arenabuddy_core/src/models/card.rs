use std::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

// Re-export the card types for easier access
pub use arenabuddy::{Card, CardCollection, CardFace};
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::models::Cost;

#[allow(clippy::all)]
mod arenabuddy {
    // Include the generated code from the build script
    include!(concat!(env!("OUT_DIR"), "/arenabuddy.rs"));
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CardType {
    Creature,
    Land,
    Artifact,
    Enchantment,
    Planeswalker,
    Instant,
    Sorcery,
    Battle,
    #[default]
    Unknown,
}
impl FromStr for CardType {
    type Err = Self;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim() {
            "Creature" => Ok(CardType::Creature),
            "Land" | "Basic Land" => Ok(CardType::Land),
            "Artifact" => Ok(CardType::Artifact),
            "Enchantment" => Ok(CardType::Enchantment),
            "Planeswalker" => Ok(CardType::Planeswalker),
            "Instant" => Ok(CardType::Instant),
            "Sorcery" => Ok(CardType::Sorcery),
            "Battle" => Ok(CardType::Battle),
            _ => Err(Self::Err::Unknown),
        }
    }
}

impl Display for CardType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}",
            match self {
                CardType::Creature => "Creature",
                CardType::Land => "Land",
                CardType::Artifact => "Artifact",
                CardType::Enchantment => "Enchantment",
                CardType::Planeswalker => "Planeswalker",
                CardType::Instant => "Instant",
                CardType::Sorcery => "Sorcery",
                CardType::Battle => "Battle",
                CardType::Unknown => "Unknown",
            }
        )
    }
}

// Utility functions for working with protobuf card types
impl Card {
    // Create a new card with required fields
    pub fn new(id: i64, set: impl Into<String>, name: impl Into<String>) -> Self {
        Self {
            id,
            set: set.into(),
            name: name.into(),
            lang: String::new(),
            image_uri: String::new(),
            mana_cost: String::new(),
            cmc: 0,
            type_line: String::new(),
            layout: String::new(),
            colors: Vec::new(),
            color_identity: Vec::new(),
            card_faces: Vec::new(),
        }
    }

    #[expect(clippy::cast_possible_truncation)]
    pub fn from_json(card_json: &serde_json::Value) -> Self {
        let mut card = Self::new(
            card_json["arena_id"].as_i64().unwrap_or_default(),
            card_json["set"].as_str().unwrap_or_default(),
            card_json["name"].as_str().unwrap_or_default(),
        );

        // Fill in optional fields if present
        if let Some(lang) = card_json["lang"].as_str() {
            card.lang = lang.to_string();
        }

        if let Some(image_uri) = card_json["image_uris"]["small"].as_str() {
            card.image_uri = image_uri.to_string();
        }

        if let Some(mana_cost) = card_json["mana_cost"].as_str() {
            card.mana_cost = mana_cost.to_string();
        }

        if let Some(cmc) = card_json["cmc"].as_f64() {
            card.cmc = cmc as i32;
        }

        if let Some(type_line) = card_json["type_line"].as_str() {
            card.type_line = type_line.to_string();
        }

        if let Some(layout) = card_json["layout"].as_str() {
            card.layout = layout.to_string();
        }

        // Parse array fields
        if let Some(colors) = card_json["colors"].as_array() {
            card.colors = colors
                .iter()
                .filter_map(|c| c.as_str().map(ToString::to_string))
                .collect();
        }

        if let Some(color_identity) = card_json["color_identity"].as_array() {
            card.color_identity = color_identity
                .iter()
                .filter_map(|c| c.as_str().map(ToString::to_string))
                .collect();
        }

        // Parse card faces if present
        if let Some(faces) = card_json["card_faces"].as_array() {
            card.card_faces = faces
                .iter()
                .filter_map(|face| {
                    if !face.is_object() {
                        return None;
                    }

                    let mut card_face = CardFace {
                        name: face["name"].as_str().unwrap_or_default().to_string(),
                        type_line: face["type_line"].as_str().unwrap_or_default().to_string(),
                        mana_cost: face["mana_cost"].as_str().unwrap_or_default().to_string(),
                        image_uri: None,
                        colors: Vec::new(),
                    };

                    // Optional fields
                    if let Some(image) = face["image_uris"]["small"].as_str() {
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
        card
    }

    pub fn mana_value(&self) -> u8 {
        self.cmc.try_into().unwrap_or(0)
    }

    pub fn cost(&self) -> Cost {
        Cost::from_str(&self.mana_cost).unwrap_or(Cost::default())
    }

    pub fn dominant_type(&self) -> Option<CardType> {
        self.type_line
            .split_whitespace()
            .next()
            .map(CardType::from_str)
            .map(|t| t.unwrap_or(CardType::Unknown))
    }

    fn multiface(&self) -> bool {
        !self.card_faces.is_empty()
    }

    pub fn primary_image_uri(&self) -> Option<String> {
        if self.multiface() {
            self.card_faces.first().and_then(|f| f.image_uri.clone())
        } else {
            Some(self.image_uri.clone())
        }
    }
}

impl Eq for Card {}

impl Ord for Card {
    fn cmp(&self, other: &Self) -> Ordering {
        let mana_value_ordering = self.mana_value().cmp(&other.mana_value());
        if mana_value_ordering == Ordering::Equal {
            self.name.cmp(&other.name)
        } else {
            mana_value_ordering
        }
    }
}

// impl PartialEq<Self> for Card {
//     fn eq(&self, other: &Self) -> bool {
//         self.cmp(other) == Ordering::Equal
//     }
// }
impl PartialOrd<Self> for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        // First write the card name and set
        write!(f, "{} ({})", self.name, self.set)?;

        // Add mana cost if available
        if !self.mana_cost.is_empty() {
            write!(f, " {}", self.mana_cost)?;
        }

        // Add type line if available
        if !self.type_line.is_empty() {
            write!(f, " - {}", self.type_line)?;
        }
        // Write the ID
        write!(f, "\nID: {}", self.id)?;

        // Write language if present
        if !self.lang.is_empty() {
            write!(f, "\nLanguage: {}", self.lang)?;
        }

        // Write image URI if present
        if !self.image_uri.is_empty() {
            write!(f, "\nImage URI: {}", self.image_uri)?;
        }

        // Write converted mana cost
        write!(f, "\nMana Value: {}", self.cmc)?;

        // Write layout if present
        if !self.layout.is_empty() {
            write!(f, "\nLayout: {}", self.layout)?;
        }

        // Write colors if present
        if !self.colors.is_empty() {
            write!(f, "\nColors: {}", self.colors.join(", "))?;
        }

        // Write color identity if present
        if !self.color_identity.is_empty() {
            write!(f, "\nColor Identity: {}", self.color_identity.join(", "))?;
        }

        // Write card faces if present
        if !self.card_faces.is_empty() {
            write!(f, "\nCard Faces:")?;
            for (i, face) in self.card_faces.iter().enumerate() {
                write!(f, "\n  Face {}: {}", i + 1, face.name)?;
                if !face.mana_cost.is_empty() {
                    write!(f, " {}", face.mana_cost)?;
                }
                if !face.type_line.is_empty() {
                    write!(f, " - {}", face.type_line)?;
                }
                if let Some(ref image) = face.image_uri {
                    write!(f, "\n    Image URI: {image}")?;
                }
                if !face.colors.is_empty() {
                    write!(f, "\n    Colors: {}", face.colors.join(", "))?;
                }
            }
        }

        Ok(())
    }
}

impl CardCollection {
    // Create a new empty card collection
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    // Add a card to the collection
    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    // Get the number of cards in the collection
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    // Check if the collection is empty
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).unwrap_or_default();
        buf
    }
}
