use std::{collections::HashMap, fmt::Write};

use itertools::Itertools;
use serde::{Deserialize, Serialize};

use crate::{
    cards::CardsDatabase,
    display::card::CardDisplayRecord,
    models::{CardType, Deck, Quantities},
};

fn get_card(db: &CardsDatabase, quantities: &Quantities, card_id: i32) -> CardDisplayRecord {
    let mut card: CardDisplayRecord = db
        .get(&card_id)
        .map_or_else(|| CardDisplayRecord::new(card_id.to_string()), std::convert::Into::into);
    card.quantity = *quantities.get(&card_id).unwrap_or(&0u16);
    card
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct DeckDisplayRecord {
    pub archetype: String,
    pub main_deck: HashMap<CardType, Vec<CardDisplayRecord>>,
    pub sideboard: Vec<CardDisplayRecord>,
}

impl DeckDisplayRecord {
    pub fn from_decklist(value: &Deck, cards_db: &CardsDatabase) -> Self {
        let archetype = "Unknown".to_string();

        let main_quantities = value.quantities();
        let sideboard_quantities = value.sideboard_quantities();

        let mut main_cards = main_quantities
            .keys()
            .copied()
            .map(|card_id| get_card(cards_db, &main_quantities, card_id))
            .fold(
                HashMap::new(),
                |mut acc: HashMap<CardType, Vec<CardDisplayRecord>>, card| {
                    let card_type = card.type_field;
                    acc.entry(card_type).or_default().push(card);
                    acc
                },
            );

        for cards in main_cards.values_mut() {
            cards.sort();
        }

        let sideboard_cards = sideboard_quantities
            .keys()
            .copied()
            .map(|card_id| get_card(cards_db, &sideboard_quantities, card_id))
            .sorted()
            .collect();

        Self {
            archetype,
            main_deck: main_cards,
            sideboard: sideboard_cards,
        }
    }

    /// Returns the total number of cards in the main deck and sideboard.
    pub fn totals(&self) -> (u16, u16) {
        (
            self.main_deck
                .values()
                .map(|c| c.iter().fold(0, |acc, card| acc + card.quantity))
                .sum(),
            self.sideboard.iter().fold(0, |acc, card| acc + card.quantity),
        )
    }

    pub fn total_by_type(&self, card_type: CardType) -> u16 {
        let Some(cards) = self.main_deck.get(&card_type) else {
            return 0;
        };
        cards.iter().fold(0, |acc, card| acc + card.quantity)
    }

    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        for (card_type, cards) in &self.main_deck {
            writeln!(output, "{card_type}:").expect("valid write");
            for card in cards {
                writeln!(output, "{} {}", card.quantity, card.name).expect("valid write");
            }
        }
        if !self.sideboard.is_empty() {
            writeln!(output, "Sideboard:").expect("valid write");
            for card in &self.sideboard {
                writeln!(output, "{} {}", card.quantity, card.name).expect("valid write");
            }
        }
        output
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Difference {
    pub added: Vec<CardDisplayRecord>,
    pub removed: Vec<CardDisplayRecord>,
}

impl Difference {
    pub fn new(added: Vec<CardDisplayRecord>, removed: Vec<CardDisplayRecord>) -> Self {
        Self { added, removed }
    }

    fn missing_cards(main1: &HashMap<i32, u16>, main2: &HashMap<i32, u16>) -> HashMap<i32, u16> {
        let mut missing = Vec::new();
        for (card_id, quantity) in main1 {
            if let Some(deck2_quantity) = main2.get(card_id) {
                if deck2_quantity < quantity {
                    let diff = quantity - deck2_quantity;
                    (0..diff).for_each(|_| missing.push(*card_id));
                }
            } else {
                (0u16..*quantity).for_each(|_| missing.push(*card_id));
            }
        }
        quantities(&missing)
    }

    fn aggregate(collection: &HashMap<i32, u16>, cards_database: &CardsDatabase) -> Vec<CardDisplayRecord> {
        collection
            .iter()
            .map(|(card_id, quantity)| -> CardDisplayRecord {
                let mut card = cards_database
                    .get(&card_id)
                    .map_or_else(|| CardDisplayRecord::new(card_id.to_string()), std::convert::Into::into);
                card.quantity = *quantity;
                card
            })
            .sorted()
            .collect()
    }

    pub fn diff(deck1: &Deck, deck2: &Deck, cards_database: &CardsDatabase) -> Self {
        let deck1_quantities = deck1.quantities();
        let deck2_quantities = deck2.quantities();

        let added = Self::missing_cards(&deck2_quantities, &deck1_quantities);
        let removed = Self::missing_cards(&deck1_quantities, &deck2_quantities);

        let added = Self::aggregate(&added, cards_database);
        let removed = Self::aggregate(&removed, cards_database);

        Self::new(added, removed)
    }

    pub fn pretty_print(&self) -> String {
        let mut output = String::new();
        if !self.added.is_empty() {
            writeln!(output, "Added:").expect("valid write");
            for card in &self.added {
                writeln!(output, "{} {}", card.quantity, card.name).expect("valid write");
            }
        }
        if !self.removed.is_empty() {
            writeln!(output, "\nRemoved:").expect("valid write");
            for card in &self.removed {
                writeln!(output, "{} {}", card.quantity, card.name).expect("valid write");
            }
        }
        output
    }
}

fn quantities(deck: &[i32]) -> HashMap<i32, u16> {
    let unique: Vec<_> = deck.iter().unique().copied().collect();
    let deck_quantities: HashMap<i32, u16> = unique
        .iter()
        .map(|ent_id| {
            let quantity = u16::try_from(deck.iter().filter(|&id| id == ent_id).count()).unwrap_or_default();
            (*ent_id, quantity)
        })
        .collect();
    deck_quantities
}
