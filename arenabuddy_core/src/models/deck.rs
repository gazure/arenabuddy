use std::{collections::HashMap, fmt::Display};

use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::mtga_events::gre::DeckMessage;

/// A mapping of card IDs to their quantities in a deck
pub type Quantities = HashMap<i32, u16>;

/// Represents a Magic: The Gathering deck
///
/// A deck consists of a name, game number, mainboard cards, and sideboard cards.
/// Card IDs are stored as integers that correspond to Arena's internal card identifiers.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deck {
    name: String,
    game_number: i32,
    mainboard: Vec<i32>,
    sideboard: Vec<i32>,
}

impl From<DeckMessage> for Deck {
    fn from(deck_message: DeckMessage) -> Self {
        Self::new(
            "Found Deck".to_string(),
            0,
            deck_message.deck_cards,
            deck_message.sideboard_cards,
        )
    }
}

impl From<&DeckMessage> for Deck {
    fn from(deck_message: &DeckMessage) -> Self {
        deck_message.clone().into()
    }
}

impl From<(String, Vec<i32>, Vec<i32>)> for Deck {
    fn from(tuple: (String, Vec<i32>, Vec<i32>)) -> Self {
        let (name, mainboard, sideboard) = tuple;
        Self::new(name, 0, mainboard, sideboard)
    }
}

impl Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\nMainboard: {} cards\n{}\nSideboard: {} cards\n{}",
            self.name(),
            self.mainboard_size(),
            &self
                .mainboard
                .iter()
                .map(ToString::to_string)
                .fold(String::new(), |acc, i| acc + &i + "\n"),
            self.sideboard_size(),
            &self
                .sideboard
                .iter()
                .map(ToString::to_string)
                .fold(String::new(), |acc, i| acc + &i + "\n")
        )
    }
}

impl Deck {
    /// Creates a new deck with the specified properties
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the deck
    /// * `game_number` - The game number this deck was used in
    /// * `mainboard` - Vector of card IDs in the mainboard
    /// * `sideboard` - Vector of card IDs in the sideboard
    ///
    /// # Returns
    ///
    /// A new Deck instance
    pub fn new(name: String, game_number: i32, mainboard: Vec<i32>, sideboard: Vec<i32>) -> Self {
        Self {
            name,
            game_number,
            mainboard,
            sideboard,
        }
    }

    /// Creates a new empty deck with the given name
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the deck
    ///
    /// # Returns
    ///
    /// A new empty Deck instance
    pub fn new_empty(name: String) -> Self {
        Self {
            name,
            game_number: 0,
            mainboard: Vec::new(),
            sideboard: Vec::new(),
        }
    }

    /// Creates a deck from raw JSON string representations
    ///
    /// # Arguments
    ///
    /// * `name` - The name of the deck
    /// * `game_number` - The game number this deck was used in
    /// * `mainboard` - JSON string of card IDs in the mainboard
    /// * `sideboard` - JSON string of card IDs in the sideboard
    ///
    /// # Returns
    ///
    /// A new Deck instance with cards parsed from the JSON strings
    pub fn from_raw(name: String, game_number: i32, mainboard: &str, sideboard: &str) -> Self {
        let mainboard = process_raw_decklist(mainboard);
        let sideboard = process_raw_decklist(sideboard);
        Self::new(name, game_number, mainboard, sideboard)
    }

    /// Returns the name of the deck
    pub fn name(&self) -> &str {
        &self.name
    }

    /// Returns the game number
    pub fn game_number(&self) -> i32 {
        self.game_number
    }

    /// Sets the game number
    pub fn set_game_number(&mut self, game_number: i32) {
        self.game_number = game_number;
    }

    /// Returns a reference to the mainboard cards
    pub fn mainboard(&self) -> &[i32] {
        &self.mainboard
    }

    /// Returns a reference to the sideboard cards
    pub fn sideboard(&self) -> &[i32] {
        &self.sideboard
    }

    /// Returns a map of card IDs to their quantities in the mainboard
    pub fn quantities(&self) -> Quantities {
        quantities(&self.mainboard)
    }

    /// Returns a map of card IDs to their quantities in the sideboard
    pub fn sideboard_quantities(&self) -> Quantities {
        quantities(&self.sideboard)
    }

    /// Returns the total number of cards in the mainboard
    pub fn mainboard_size(&self) -> usize {
        self.mainboard.len()
    }

    /// Returns the total number of cards in the sideboard
    pub fn sideboard_size(&self) -> usize {
        self.sideboard.len()
    }

    /// Returns the number of unique cards in the mainboard
    pub fn unique_mainboard_cards(&self) -> usize {
        self.quantities().len()
    }

    /// Returns the number of unique cards in the sideboard
    pub fn unique_sideboard_cards(&self) -> usize {
        self.sideboard_quantities().len()
    }

    /// Adds a card to the mainboard
    pub fn add_to_mainboard(&mut self, card_id: i32) {
        self.mainboard.push(card_id);
    }

    /// Adds a card to the sideboard
    pub fn add_to_sideboard(&mut self, card_id: i32) {
        self.sideboard.push(card_id);
    }

    /// Adds multiple copies of a card to the mainboard
    pub fn add_copies_to_mainboard(&mut self, card_id: i32, count: usize) {
        self.mainboard.extend(std::iter::repeat_n(card_id, count));
    }

    /// Adds multiple copies of a card to the sideboard
    pub fn add_copies_to_sideboard(&mut self, card_id: i32, count: usize) {
        self.sideboard.extend(std::iter::repeat_n(card_id, count));
    }

    /// Returns whether the mainboard contains at least one copy of the specified card
    pub fn mainboard_contains(&self, card_id: i32) -> bool {
        self.mainboard.contains(&card_id)
    }

    /// Returns whether the sideboard contains at least one copy of the specified card
    pub fn sideboard_contains(&self, card_id: i32) -> bool {
        self.sideboard.contains(&card_id)
    }
}

pub fn quantities(deck: &[i32]) -> Quantities {
    deck.iter()
        .unique()
        .copied()
        .map(|card_id| {
            let quantity =
                u16::try_from(deck.iter().filter(|&id| *id == card_id).count()).unwrap_or(0);
            (card_id, quantity)
        })
        .collect()
}

fn process_raw_decklist(raw_decklist: &str) -> Vec<i32> {
    let parsed = serde_json::from_str(raw_decklist).unwrap_or(Value::Array(Vec::new()));
    if let Value::Array(arr) = parsed {
        arr.iter()
            .filter_map(Value::as_i64)
            .filter_map(|v| i32::try_from(v).ok())
            .collect()
    } else {
        Vec::default()
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_quantities() {
        let deck = vec![1, 2, 3, 1, 2, 3, 1, 2, 3, 4];
        let quantities = super::quantities(&deck);
        assert_eq!(quantities.get(&1), Some(&3));
        assert_eq!(quantities.get(&2), Some(&3));
        assert_eq!(quantities.get(&3), Some(&3));
        assert_eq!(quantities.get(&4), Some(&1));
    }

    #[test]
    fn test_process_raw_decklist() {
        let raw_decklist = "[1, 2, 3, 4]";
        let deck = super::process_raw_decklist(raw_decklist);
        assert_eq!(deck, vec![1, 2, 3, 4]);
    }

    #[test]
    fn test_deck_from_raw_decklist() {
        let deck = super::Deck::from_raw("Test Deck".to_string(), 0, "[1, 2, 3]", "[4, 5, 6]");
        assert_eq!(deck.name, "Test Deck");
        assert_eq!(deck.game_number, 0);
        assert_eq!(deck.mainboard, vec![1, 2, 3]);
        assert_eq!(deck.sideboard, vec![4, 5, 6]);
    }

    #[test]
    fn test_deck_display() {
        let deck = super::Deck::new("Test Deck".to_string(), 0, vec![1, 2, 3], vec![4, 5, 6]);
        let display = format!("{deck}");
        assert_eq!(
            display,
            "Test Deck\nMainboard: 3 cards\n1\n2\n3\n\nSideboard: 3 cards\n4\n5\n6\n"
        );
    }

    #[test]
    fn test_deck_from_deck_message() {
        let deck_message = crate::mtga_events::gre::DeckMessage {
            deck_cards: vec![1, 2, 3],
            sideboard_cards: vec![4, 5, 6],
        };
        let deck = super::Deck::from(deck_message);
        assert_eq!(deck.name, "Found Deck");
        assert_eq!(deck.game_number, 0);
        assert_eq!(deck.mainboard, vec![1, 2, 3]);
        assert_eq!(deck.sideboard, vec![4, 5, 6]);
    }

    #[test]
    fn test_deck_from_deck_message_ref() {
        let deck_message = crate::mtga_events::gre::DeckMessage {
            deck_cards: vec![1, 2, 3],
            sideboard_cards: vec![4, 5, 6],
        };
        let deck = super::Deck::from(&deck_message);
        assert_eq!(deck.name, "Found Deck");
        assert_eq!(deck.game_number, 0);
        assert_eq!(deck.mainboard, vec![1, 2, 3]);
        assert_eq!(deck.sideboard, vec![4, 5, 6]);
    }

    #[test]
    fn test_deck_quantities() {
        let deck = super::Deck::new(
            "Test Deck".to_string(),
            0,
            vec![1, 2, 3, 1, 2, 3, 1, 2, 3, 4],
            vec![4, 5, 6],
        );
        let quantities = deck.quantities();
        assert_eq!(quantities.get(&1), Some(&3));
        assert_eq!(quantities.get(&2), Some(&3));
        assert_eq!(quantities.get(&3), Some(&3));
        assert_eq!(quantities.get(&4), Some(&1));
    }

    #[test]
    fn test_deck_sideboard_quantities() {
        let deck = super::Deck::new(
            "Test Deck".to_string(),
            0,
            vec![1, 2, 3, 1, 2, 3, 1, 2, 3, 4],
            vec![4, 5, 6],
        );
        let quantities = deck.sideboard_quantities();
        assert_eq!(quantities.get(&4), Some(&1));
        assert_eq!(quantities.get(&5), Some(&1));
        assert_eq!(quantities.get(&6), Some(&1));
    }
}
