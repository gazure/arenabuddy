use serde::{Deserialize, Serialize};

use crate::{
    cards::{Card, CardType},
    proto::Card as ProtoCard,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CardDisplayRecord {
    pub name: String,
    pub type_field: CardType,
    pub mana_value: u8,
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
}

impl Default for CardDisplayRecord {
    fn default() -> Self {
        Self {
            name: "Unknown".to_string(),
            type_field: CardType::Unknown,
            mana_value: 0,
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

impl From<&ProtoCard> for CardDisplayRecord {
    fn from(value: &ProtoCard) -> Self {
        let name = if value.card_faces.is_empty() {
            value.name.clone()
        } else {
            let front_face = &value.card_faces[0];
            front_face.name.clone()
        };

        Self {
            name,
            type_field: value.dominant_type().unwrap_or(CardType::Unknown),
            mana_value: value.mana_value(),
            quantity: 1,
            image_uri: value.image_uri.clone(),
        }
    }
}

impl From<&Card> for CardDisplayRecord {
    fn from(entry: &Card) -> Self {
        let name = if let Some(card_faces) = &entry.card_faces {
            let front_face = &card_faces[0];
            front_face.name.clone()
        } else {
            entry.name.clone()
        };
        let image_uri = if let Some(image_uri) = &entry.image_uri {
            image_uri.clone()
        } else {
            entry
                .card_faces
                .as_ref()
                .and_then(|faces| faces.first().map(|face| face.image_uri.clone()))
                .flatten()
                .as_ref()
                .unwrap_or(&String::new())
                .to_string()
        };

        Self {
            name,
            type_field: entry.dominant_type(),
            #[allow(clippy::cast_possible_truncation)]
            mana_value: entry.cmc,
            quantity: 1,
            image_uri: image_uri.clone(),
        }
    }
}
