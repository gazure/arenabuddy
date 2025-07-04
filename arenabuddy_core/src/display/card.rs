use serde::{Deserialize, Serialize};

use crate::models::{Card, CardType, Cost};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDisplayRecord {
    pub name: String,
    pub type_field: CardType,
    pub mana_value: u8,
    pub mana: String,
    pub quantity: u16,
    pub image_uri: String,
}

impl CardDisplayRecord {
    pub fn new(name: String) -> Self {
        Self {
            name,
            ..Default::default()
        }
    }

    pub fn cost(&self) -> Cost {
        self.mana.parse().unwrap_or_default()
    }
}

impl Default for CardDisplayRecord {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            type_field: CardType::Unknown,
            mana_value: 0,
            mana: String::new(),
            quantity: 0,
            image_uri: String::new(),
        }
    }
}

impl Eq for CardDisplayRecord {}

impl PartialEq for CardDisplayRecord {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Ord for CardDisplayRecord {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for CardDisplayRecord {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl From<&Card> for CardDisplayRecord {
    fn from(value: &Card) -> Self {
        let name = if value.card_faces.is_empty() {
            value.name.clone()
        } else {
            let front_face = &value.card_faces[0];
            front_face.name.clone()
        };

        Self {
            name,
            type_field: value.dominant_type(),
            mana_value: value.mana_value(),
            mana: value.mana_cost.clone(),
            quantity: 1,
            image_uri: value.image_uri.clone(),
        }
    }
}
