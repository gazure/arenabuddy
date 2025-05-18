
pub mod arenabuddy {
    // Include the generated code from the build script
    include!(concat!(env!("OUT_DIR"), "/arenabuddy.rs"));
}

// Re-export the card types for easier access
pub use arenabuddy::{Card, CardCollection, CardFace};

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

    // Convert from JSON representation
    pub fn from_json(json: &str) -> Result<Self, prost::DecodeError> {
        let card_json: serde_json::Value = serde_json::from_str(json)
            .map_err(|_| prost::DecodeError::new("Failed to parse JSON"))?;

        let mut card = Self::new(
            card_json["id"].as_i64().unwrap_or_default(),
            card_json["set"].as_str().unwrap_or_default(),
            card_json["name"].as_str().unwrap_or_default(),
        );

        // Fill in optional fields if present
        if let Some(lang) = card_json["lang"].as_str() {
            card.lang = lang.to_string();
        }

        if let Some(image_uri) = card_json["image_uri"].as_str() {
            card.image_uri = image_uri.to_string();
        }

        if let Some(mana_cost) = card_json["mana_cost"].as_str() {
            card.mana_cost = mana_cost.to_string();
        }

        if let Some(cmc) = card_json["cmc"].as_i64() {
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
                        name: face["name"].as_str()?.to_string(),
                        type_line: face["type_line"].as_str()?.to_string(),
                        mana_cost: face["mana_cost"].as_str()?.to_string(),
                        image_uri: None,
                        colors: Vec::new(),
                    };

                    // Optional fields
                    if let Some(image) = face["image_uri"].as_str() {
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

        Ok(card)
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
}
