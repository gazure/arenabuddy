use std::{collections::HashMap, fmt::Display};

use itertools::Itertools;
use serde_json::Value;

use crate::events::gre::DeckMessage;
/// Re-export the Deck type from proto
///
/// A deck consists of a name, game number, mainboard cards, and sideboard cards.
/// Card IDs are stored as integers that correspond to Arena's internal card identifiers.
pub use crate::proto::Deck;

/// A mapping of card IDs to their quantities in a deck
pub struct Quantities(HashMap<i32, usize>);

impl Quantities {
    pub fn from_cards(deck: &[i32]) -> Self {
        let unique: Vec<_> = deck.iter().unique().copied().collect();
        let deck_quantities = unique
            .iter()
            .map(|ent_id| {
                let quantity = deck.iter().filter(|&id| id == ent_id).count();
                (*ent_id, quantity)
            })
            .collect();

        Quantities(deck_quantities)
    }

    pub fn get(&self, card_id: i32) -> Option<usize> {
        self.0.get(&card_id).copied()
    }

    pub fn keys(&self) -> impl Iterator<Item = i32> {
        self.0.keys().copied()
    }

    pub fn iter(&self) -> impl Iterator<Item = (i32, usize)> + '_ {
        self.0.iter().map(|(c, q)| (*c, *q))
    }
}

impl<'a> IntoIterator for &'a Quantities {
    type IntoIter =
        std::iter::Map<std::collections::hash_map::Iter<'a, i32, usize>, fn((&'a i32, &'a usize)) -> (i32, usize)>;
    type Item = (i32, usize);

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(c, q)| (*c, *q))
    }
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

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn game_number(&self) -> i32 {
        self.game_number
    }

    pub fn set_game_number(&mut self, game_number: i32) {
        self.game_number = game_number;
    }

    pub fn mainboard(&self) -> &[i32] {
        &self.mainboard
    }

    pub fn sideboard(&self) -> &[i32] {
        &self.sideboard
    }

    pub fn quantities(&self) -> Quantities {
        Quantities::from_cards(&self.mainboard)
    }

    pub fn sideboard_quantities(&self) -> Quantities {
        Quantities::from_cards(&self.sideboard)
    }

    fn mainboard_size(&self) -> usize {
        self.mainboard.len()
    }

    fn sideboard_size(&self) -> usize {
        self.sideboard.len()
    }
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
        let quantities = super::Quantities::from_cards(&deck);
        assert_eq!(quantities.get(1), Some(3));
        assert_eq!(quantities.get(2), Some(3));
        assert_eq!(quantities.get(3), Some(3));
        assert_eq!(quantities.get(4), Some(1));
    }

    #[test]
    fn test_process_raw_decklist() {
        let raw_decklist = "[1, 2, 3, 4]";
        let deck = super::process_raw_decklist(raw_decklist);
        assert_eq!(deck, vec![1, 2, 3, 4]);
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
        let deck_message = crate::events::gre::DeckMessage {
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
        let deck_message = crate::events::gre::DeckMessage {
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
        assert_eq!(quantities.get(1), Some(3));
        assert_eq!(quantities.get(2), Some(3));
        assert_eq!(quantities.get(3), Some(3));
        assert_eq!(quantities.get(4), Some(1));
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
        assert_eq!(quantities.get(4), Some(1));
        assert_eq!(quantities.get(5), Some(1));
        assert_eq!(quantities.get(6), Some(1));
    }
}
