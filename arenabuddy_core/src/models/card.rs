use std::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

// Re-export the card types for easier access
/// Re-export the card types for easier access
/// These are protobuf-generated types that represent Magic: The Gathering cards
pub use arenabuddy::{Card, CardCollection, CardFace};
use prost::Message;
use serde::{Deserialize, Serialize};

use crate::models::Cost;

#[allow(clippy::all)]
mod arenabuddy {
    // Include the generated code from the build script
    include!(concat!(env!("OUT_DIR"), "/arenabuddy.rs"));
}

/// Represents the primary type of a Magic: The Gathering card
///
/// Each card in Magic has one or more types that define its characteristics
/// and how it can be played. This enum represents the most common primary types.
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
    /// Creates a new card with required fields, initializing optional fields to empty values
    ///
    /// # Arguments
    ///
    /// * `id` - The Arena ID of the card
    /// * `set` - The set code the card belongs to (e.g., "RNA" for Ravnica Allegiance)
    /// * `name` - The name of the card
    ///
    /// # Returns
    ///
    /// A new Card instance with minimal initialization
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

    /// Returns the card's ID
    pub fn id(&self) -> i64 {
        self.id
    }

    /// Returns the card's set code
    pub fn set(&self) -> &str {
        &self.set
    }

    /// Returns the card's name
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the card's language
    pub fn lang(&self) -> &str {
        &self.lang
    }

    /// Returns the card's mana cost string
    pub fn mana_cost_str(&self) -> &str {
        &self.mana_cost
    }

    /// Returns the card's type line
    pub fn type_line(&self) -> &str {
        &self.type_line
    }

    /// Returns the card's layout
    pub fn layout(&self) -> &str {
        &self.layout
    }

    /// Returns the card's colors
    pub fn colors(&self) -> &[String] {
        &self.colors
    }

    /// Returns the card's color identity
    pub fn color_identity(&self) -> &[String] {
        &self.color_identity
    }

    /// Returns the card's faces, if it has multiple faces
    pub fn faces(&self) -> &[CardFace] {
        &self.card_faces
    }

    /// Returns the card's mana value (formerly known as converted mana cost)
    ///
    /// # Returns
    ///
    /// The total mana value of the card as a u8
    pub fn mana_value(&self) -> u8 {
        self.cmc.try_into().unwrap_or(0)
    }

    /// Returns the card's mana cost as a structured Cost object
    ///
    /// # Returns
    ///
    /// A Cost object representing the mana cost, or the default Cost if parsing fails
    pub fn cost(&self) -> Cost {
        Cost::from_str(&self.mana_cost).unwrap_or(Cost::default())
    }

    /// Determines the dominant card type from the type line
    ///
    /// # Returns
    ///
    /// The primary `CardType` of this card, or None if it couldn't be determined
    pub fn dominant_type(&self) -> CardType {
        // Handle basic lands explicitly
        if self.type_line.contains("Basic Land") {
            return CardType::Land;
        }

        self.type_line
            .split_whitespace()
            .find_map(|s| CardType::from_str(s).ok())
            .unwrap_or(CardType::Unknown)
    }

    /// Checks if this card has multiple faces
    ///
    /// # Returns
    ///
    /// true if the card has multiple faces, false otherwise
    fn multiface(&self) -> bool {
        !self.card_faces.is_empty()
    }

    /// Gets the primary image URI for the card
    ///
    /// For single-faced cards, this is the main image URI.
    /// For multi-faced cards, this is the image URI of the first face.
    ///
    /// # Returns
    ///
    /// The image URI as an Option<String>
    pub fn primary_image_uri(&self) -> Option<&str> {
        if self.multiface() {
            self.card_faces.first().and_then(|f| f.image_uri.as_deref())
        } else {
            Some(&self.image_uri)
        }
    }

    /// Checks if the card matches the given color
    ///
    /// # Arguments
    ///
    /// * `color` - The color to check for
    ///
    /// # Returns
    ///
    /// true if the card contains the specified color, false otherwise
    pub fn has_color(&self, color: &str) -> bool {
        self.colors.iter().any(|c| c == color)
    }

    /// Checks if the card is multicolored
    ///
    /// # Returns
    ///
    /// true if the card has more than one color, false otherwise
    pub fn is_multicolored(&self) -> bool {
        self.colors.len() > 1
    }

    /// Checks if the card is colorless
    ///
    /// # Returns
    ///
    /// true if the card has no colors, false otherwise
    pub fn is_colorless(&self) -> bool {
        self.colors.is_empty()
    }

    /// Checks if this card is of the specified type
    ///
    /// # Arguments
    ///
    /// * `card_type` - The card type to check for
    ///
    /// # Returns
    ///
    /// true if the card's type line contains the specified type, false otherwise
    pub fn is_type(&self, card_type: &str) -> bool {
        self.type_line.contains(card_type)
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
    /// Creates a new empty card collection
    pub fn new() -> Self {
        Self { cards: Vec::new() }
    }

    /// Creates a new card collection with the specified cards
    ///
    /// # Arguments
    ///
    /// * `cards` - A vector of Card objects to initialize the collection with
    ///
    /// # Returns
    ///
    /// A new `CardCollection` containing the specified cards
    pub fn with_cards(cards: Vec<Card>) -> Self {
        Self { cards }
    }

    /// Adds a card to the collection
    ///
    /// # Arguments
    ///
    /// * `card` - The Card to add to the collection
    pub fn add_card(&mut self, card: Card) {
        self.cards.push(card);
    }

    /// Adds multiple cards to the collection
    ///
    /// # Arguments
    ///
    /// * `cards` - A slice of Card objects to add to the collection
    pub fn add_cards(&mut self, cards: &[Card]) {
        self.cards.extend_from_slice(cards);
    }

    /// Removes a card from the collection by index
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the card to remove
    ///
    /// # Returns
    ///
    /// The removed Card if the index was valid, None otherwise
    pub fn remove_card(&mut self, index: usize) -> Option<Card> {
        if index < self.cards.len() {
            Some(self.cards.remove(index))
        } else {
            None
        }
    }

    /// Gets a reference to the cards in this collection
    ///
    /// # Returns
    ///
    /// A slice containing references to all cards in the collection
    pub fn cards(&self) -> &[Card] {
        &self.cards
    }

    /// Gets a reference to a specific card by index
    ///
    /// # Arguments
    ///
    /// * `index` - The index of the card to retrieve
    ///
    /// # Returns
    ///
    /// A reference to the Card at the specified index, or None if the index is out of bounds
    pub fn get(&self, index: usize) -> Option<&Card> {
        self.cards.get(index)
    }

    /// Finds a card by its Arena ID
    ///
    /// # Arguments
    ///
    /// * `id` - The Arena ID to search for
    ///
    /// # Returns
    ///
    /// A reference to the first Card with the specified ID, or None if no matching card is found
    pub fn find_by_id(&self, id: i64) -> Option<&Card> {
        self.cards.iter().find(|card| card.id == id)
    }

    /// Finds all cards with the specified name
    ///
    /// # Arguments
    ///
    /// * `name` - The name to search for
    ///
    /// # Returns
    ///
    /// A vector of references to Cards with the specified name
    pub fn find_by_name(&self, name: &str) -> Vec<&Card> {
        self.cards.iter().filter(|card| card.name == name).collect()
    }

    /// Finds all cards from the specified set
    ///
    /// # Arguments
    ///
    /// * `set` - The set code to search for
    ///
    /// # Returns
    ///
    /// A vector of references to Cards from the specified set
    pub fn find_by_set(&self, set: &str) -> Vec<&Card> {
        self.cards.iter().filter(|card| card.set == set).collect()
    }

    /// Gets the number of cards in the collection
    ///
    /// # Returns
    ///
    /// The number of cards in the collection
    pub fn len(&self) -> usize {
        self.cards.len()
    }

    /// Checks if the collection is empty
    ///
    /// # Returns
    ///
    /// true if the collection contains no cards, false otherwise
    pub fn is_empty(&self) -> bool {
        self.cards.is_empty()
    }

    /// Sorts the cards in this collection by mana value, then by name
    pub fn sort(&mut self) {
        self.cards.sort();
    }

    /// Encodes the card collection to a vector of bytes using Protocol Buffers
    ///
    /// # Returns
    ///
    /// A vector of bytes representing the serialized `CardCollection`
    pub fn encode_to_vec(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        self.encode(&mut buf).unwrap_or_default();
        buf
    }
}
