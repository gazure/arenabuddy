use std::{
    cmp::Ordering,
    fmt::{Display, Formatter, Result as FmtResult},
    str::FromStr,
};

use prost::Message;
use serde::{Deserialize, Serialize};

use crate::models::Cost;
/// Re-export the card types from proto - these ARE our domain types
///
/// In Rust with prost, proto types can have methods and traits implemented directly on them,
/// so there's no need for separate wrapper types. This is different from Go where you typically
/// need wrappers to add methods to proto-generated structs.
pub use crate::proto::{Card, CardCollection, CardFace, Legalities};

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

impl CardType {
    pub fn iter() -> impl Iterator<Item = CardType> {
        [
            CardType::Creature,
            CardType::Planeswalker,
            CardType::Artifact,
            CardType::Enchantment,
            CardType::Instant,
            CardType::Sorcery,
            CardType::Battle,
            CardType::Land,
            CardType::Unknown,
        ]
        .into_iter()
    }
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

// Domain logic implementations on proto types
// This is idiomatic Rust - we can add methods directly to proto-generated types

/// Extracts a JSON array of strings, returning an empty Vec for anything else
fn str_array(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|a| a.iter().filter_map(|v| v.as_str().map(ToString::to_string)).collect())
        .unwrap_or_default()
}

impl CardFace {
    /// Parses a single face from a Scryfall `card_faces` entry, returning None for non-objects
    fn from_json(face: &serde_json::Value) -> Option<Self> {
        if !face.is_object() {
            return None;
        }

        Some(Self {
            name: face["name"].as_str().unwrap_or_default().to_string(),
            type_line: face["type_line"].as_str().unwrap_or_default().to_string(),
            mana_cost: face["mana_cost"].as_str().unwrap_or_default().to_string(),
            image_uri: face["image_uris"]["normal"].as_str().map(ToString::to_string),
            colors: str_array(&face["colors"]),
            oracle_text: face["oracle_text"].as_str().unwrap_or_default().to_string(),
            power: face["power"].as_str().map(ToString::to_string),
            toughness: face["toughness"].as_str().map(ToString::to_string),
            loyalty: face["loyalty"].as_str().map(ToString::to_string),
            defense: face["defense"].as_str().map(ToString::to_string),
            flavor_text: face["flavor_text"].as_str().unwrap_or_default().to_string(),
            artist: face["artist"].as_str().unwrap_or_default().to_string(),
        })
    }
}

impl Legalities {
    /// Parses a Scryfall `legalities` object, returning None for non-objects
    fn from_json(value: &serde_json::Value) -> Option<Self> {
        let legalities = value.as_object()?;
        let legality = |format: &str| {
            legalities
                .get(format)
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string()
        };
        Some(Self {
            standard: legality("standard"),
            alchemy: legality("alchemy"),
            historic: legality("historic"),
            timeless: legality("timeless"),
            brawl: legality("brawl"),
            standard_brawl: legality("standardbrawl"),
            gladiator: legality("gladiator"),
            pioneer: legality("pioneer"),
        })
    }
}

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
            ..Self::default()
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

        if let Some(image_uri) = card_json["image_uris"]["normal"].as_str() {
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
        card.colors = str_array(&card_json["colors"]);
        card.color_identity = str_array(&card_json["color_identity"]);
        card.keywords = str_array(&card_json["keywords"]);
        card.produced_mana = str_array(&card_json["produced_mana"]);

        if let Some(oracle_text) = card_json["oracle_text"].as_str() {
            card.oracle_text = oracle_text.to_string();
        }

        card.power = card_json["power"].as_str().map(ToString::to_string);
        card.toughness = card_json["toughness"].as_str().map(ToString::to_string);
        card.loyalty = card_json["loyalty"].as_str().map(ToString::to_string);
        card.defense = card_json["defense"].as_str().map(ToString::to_string);

        if let Some(rarity) = card_json["rarity"].as_str() {
            card.rarity = rarity.to_string();
        }

        if let Some(collector_number) = card_json["collector_number"].as_str() {
            card.collector_number = collector_number.to_string();
        }

        if let Some(set_name) = card_json["set_name"].as_str() {
            card.set_name = set_name.to_string();
        }

        if let Some(artist) = card_json["artist"].as_str() {
            card.artist = artist.to_string();
        }

        if let Some(flavor_text) = card_json["flavor_text"].as_str() {
            card.flavor_text = flavor_text.to_string();
        }

        card.legalities = Legalities::from_json(&card_json["legalities"]);

        card.edhrec_rank = card_json["edhrec_rank"].as_i64().map(|r| r as i32);
        card.penny_rank = card_json["penny_rank"].as_i64().map(|r| r as i32);

        if let Some(oracle_id) = card_json["oracle_id"].as_str() {
            card.oracle_id = oracle_id.to_string();
        }

        if let Some(scryfall_id) = card_json["id"].as_str() {
            card.scryfall_id = scryfall_id.to_string();
        }

        if let Some(scryfall_uri) = card_json["scryfall_uri"].as_str() {
            card.scryfall_uri = scryfall_uri.to_string();
        }

        // Parse card faces if present
        if let Some(faces) = card_json["card_faces"].as_array() {
            card.card_faces = faces.iter().filter_map(CardFace::from_json).collect();
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

    /// Returns the card's oracle rules text
    pub fn oracle_text(&self) -> &str {
        &self.oracle_text
    }

    // Note: accessors for `power`, `toughness`, `loyalty`, and `defense` are
    // generated by prost (optional proto3 fields), returning "" when unset.

    /// Returns the card's keyword abilities
    pub fn keywords(&self) -> &[String] {
        &self.keywords
    }

    /// Returns the mana colors this card can produce
    pub fn produced_mana(&self) -> &[String] {
        &self.produced_mana
    }

    /// Returns the card's rarity
    pub fn rarity(&self) -> &str {
        &self.rarity
    }

    /// Returns the card's collector number
    pub fn collector_number(&self) -> &str {
        &self.collector_number
    }

    /// Returns the full name of the card's set
    pub fn set_name(&self) -> &str {
        &self.set_name
    }

    /// Returns the card's artist
    pub fn artist(&self) -> &str {
        &self.artist
    }

    /// Returns the card's flavor text
    pub fn flavor_text(&self) -> &str {
        &self.flavor_text
    }

    /// Returns the card's format legalities, if known
    pub fn legalities(&self) -> Option<&Legalities> {
        self.legalities.as_ref()
    }

    /// Checks whether the card is legal in the given Arena-relevant format
    ///
    /// Format names match Scryfall's keys: "standard", "alchemy", "historic",
    /// "timeless", "brawl", "standardbrawl", "gladiator", "pioneer"
    pub fn is_legal_in(&self, format: &str) -> bool {
        let Some(legalities) = &self.legalities else {
            return false;
        };
        let status = match format {
            "standard" => &legalities.standard,
            "alchemy" => &legalities.alchemy,
            "historic" => &legalities.historic,
            "timeless" => &legalities.timeless,
            "brawl" => &legalities.brawl,
            "standardbrawl" => &legalities.standard_brawl,
            "gladiator" => &legalities.gladiator,
            "pioneer" => &legalities.pioneer,
            _ => return false,
        };
        status == "legal" || status == "restricted"
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

impl PartialOrd<Self> for Card {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Display for Card {
    fn fmt(&self, f: &mut Formatter<'_>) -> FmtResult {
        write!(f, "{} ({})", self.name, self.set)?;

        if !self.mana_cost.is_empty() {
            write!(f, " {}", self.mana_cost)?;
        }

        if !self.type_line.is_empty() {
            write!(f, " - {}", self.type_line)?;
        }
        write!(f, "\nID: {}", self.id)?;

        if !self.lang.is_empty() {
            write!(f, "\nLanguage: {}", self.lang)?;
        }

        if !self.image_uri.is_empty() {
            write!(f, "\nImage URI: {}", self.image_uri)?;
        }

        write!(f, "\nMana Value: {}", self.cmc)?;

        if !self.rarity.is_empty() {
            write!(f, "\nRarity: {}", self.rarity)?;
        }

        if let (Some(power), Some(toughness)) = (&self.power, &self.toughness) {
            write!(f, "\nPower/Toughness: {power}/{toughness}")?;
        }

        if !self.oracle_text.is_empty() {
            write!(f, "\nOracle Text: {}", self.oracle_text)?;
        }

        if !self.keywords.is_empty() {
            write!(f, "\nKeywords: {}", self.keywords.join(", "))?;
        }

        if !self.layout.is_empty() {
            write!(f, "\nLayout: {}", self.layout)?;
        }

        if !self.colors.is_empty() {
            write!(f, "\nColors: {}", self.colors.join(", "))?;
        }

        if !self.color_identity.is_empty() {
            write!(f, "\nColor Identity: {}", self.color_identity.join(", "))?;
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_json_hydrates_scryfall_fields() {
        let json = serde_json::json!({
            "arena_id": 12345,
            "id": "88b13bc0-da54-4c3b-917c-7c8345a329f5",
            "oracle_id": "f34b9bc4-7bfe-47fd-ba23-4eeeb46026eb",
            "name": "Grizzly Bears",
            "set": "lea",
            "set_name": "Limited Edition Alpha",
            "lang": "en",
            "collector_number": "142",
            "rarity": "common",
            "mana_cost": "{1}{G}",
            "cmc": 2.0,
            "type_line": "Creature — Bear",
            "layout": "normal",
            "oracle_text": "A bear.",
            "flavor_text": "Don't try to outrun one.",
            "artist": "Jeff A. Menges",
            "power": "2",
            "toughness": "2",
            "keywords": ["Trample"],
            "produced_mana": ["G"],
            "colors": ["G"],
            "color_identity": ["G"],
            "edhrec_rank": 2430,
            "penny_rank": 1392,
            "scryfall_uri": "https://scryfall.com/card/lea/142/grizzly-bears",
            "legalities": {
                "standard": "not_legal",
                "alchemy": "not_legal",
                "historic": "legal",
                "timeless": "legal",
                "brawl": "legal",
                "standardbrawl": "not_legal",
                "gladiator": "legal",
                "pioneer": "banned"
            }
        });

        let card = Card::from_json(&json);

        assert_eq!(card.id(), 12345);
        assert_eq!(card.oracle_text(), "A bear.");
        assert_eq!(card.power(), "2");
        assert_eq!(card.toughness(), "2");
        assert_eq!(card.loyalty, None);
        assert_eq!(card.keywords(), ["Trample"]);
        assert_eq!(card.produced_mana(), ["G"]);
        assert_eq!(card.rarity(), "common");
        assert_eq!(card.collector_number(), "142");
        assert_eq!(card.set_name(), "Limited Edition Alpha");
        assert_eq!(card.artist(), "Jeff A. Menges");
        assert_eq!(card.flavor_text(), "Don't try to outrun one.");
        assert_eq!(card.oracle_id, "f34b9bc4-7bfe-47fd-ba23-4eeeb46026eb");
        assert_eq!(card.scryfall_id, "88b13bc0-da54-4c3b-917c-7c8345a329f5");
        assert_eq!(card.scryfall_uri, "https://scryfall.com/card/lea/142/grizzly-bears");
        assert_eq!(card.edhrec_rank, Some(2430));
        assert_eq!(card.penny_rank, Some(1392));

        let legalities = card.legalities().expect("legalities should be parsed");
        assert_eq!(legalities.historic, "legal");
        assert_eq!(legalities.standard, "not_legal");
        assert!(card.is_legal_in("timeless"));
        assert!(!card.is_legal_in("standard"));
        assert!(!card.is_legal_in("pioneer"));
        assert!(!card.is_legal_in("modern"));
    }

    #[test]
    fn test_from_json_hydrates_card_faces() {
        let json = serde_json::json!({
            "arena_id": 555,
            "name": "Aberrant // Aberrant",
            "set": "dsk",
            "layout": "transform",
            "card_faces": [
                {
                    "name": "Front",
                    "type_line": "Creature — Horror",
                    "mana_cost": "{2}{B}",
                    "oracle_text": "Menace",
                    "power": "3",
                    "toughness": "1",
                    "flavor_text": "It hungers.",
                    "artist": "Someone",
                    "colors": ["B"],
                    "image_uris": {"normal": "https://example.com/front.jpg"}
                },
                {
                    "name": "Back",
                    "type_line": "Creature — Elder Horror",
                    "mana_cost": "",
                    "oracle_text": "Menace, deathtouch",
                    "power": "6",
                    "toughness": "5",
                    "colors": ["B"]
                }
            ]
        });

        let card = Card::from_json(&json);

        assert_eq!(card.faces().len(), 2);
        let front = &card.faces()[0];
        assert_eq!(front.oracle_text, "Menace");
        assert_eq!(front.power(), "3");
        assert_eq!(front.toughness(), "1");
        assert_eq!(front.flavor_text, "It hungers.");
        assert_eq!(front.artist, "Someone");
        assert_eq!(front.image_uri.as_deref(), Some("https://example.com/front.jpg"));

        let back = &card.faces()[1];
        assert_eq!(back.oracle_text, "Menace, deathtouch");
        assert_eq!(back.power(), "6");
        assert!(back.image_uri.is_none());
        assert!(back.flavor_text.is_empty());
    }
}
