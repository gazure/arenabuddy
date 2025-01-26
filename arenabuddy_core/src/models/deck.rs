use itertools::Itertools;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt::Display;

use crate::mtga_events::gre::DeckMessage;

pub type Quantities = HashMap<i32, u16>;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Deck {
    pub name: String,
    pub game_number: i32,
    pub mainboard: Vec<i32>,
    pub sideboard: Vec<i32>,
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

impl Display for Deck {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}\nMainboard:\n{}\nSideboard:\n{}",
            self.name,
            &self
                .mainboard
                .iter()
                .map(ToString::to_string)
                .fold(String::new(), |acc, i| acc + &i + "\n"),
            &self
                .sideboard
                .iter()
                .map(ToString::to_string)
                .fold(String::new(), |acc, i| acc + &i + "\n")
        )
    }
}

impl Deck {
    pub fn new(name: String, game_number: i32, mainboard: Vec<i32>, sideboard: Vec<i32>) -> Self {
        Self {
            name,
            game_number,
            mainboard,
            sideboard,
        }
    }

    pub fn from_raw_decklist(
        name: String,
        game_number: i32,
        mainboard: &str,
        sideboard: &str,
    ) -> Self {
        let mainboard = process_raw_decklist(mainboard);
        let sideboard = process_raw_decklist(sideboard);
        Self::new(name, game_number, mainboard, sideboard)
    }

    pub fn quantities(&self) -> Quantities {
        quantities(&self.mainboard)
    }

    pub fn sideboard_quantities(&self) -> Quantities {
        quantities(&self.sideboard)
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
    match parsed {
        Value::Array(arr) => arr
            .iter()
            .filter_map(Value::as_i64)
            .filter_map(|v| i32::try_from(v).ok())
            .collect(),
        _ => Vec::new(),
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
        let deck =
            super::Deck::from_raw_decklist("Test Deck".to_string(), 0, "[1, 2, 3]", "[4, 5, 6]");
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
            "Test Deck\nMainboard:\n1\n2\n3\n\nSideboard:\n4\n5\n6\n"
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
