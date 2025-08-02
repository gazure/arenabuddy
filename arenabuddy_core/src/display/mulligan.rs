use serde::{Deserialize, Serialize};

use crate::{
    cards::CardsDatabase, display::card::CardDisplayRecord, models::Mulligan as ModelMulligan,
};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct Mulligan {
    pub hand: Vec<CardDisplayRecord>,
    pub opponent_identity: String,
    pub game_number: i32,
    pub number_to_keep: i32,
    pub play_draw: String,
    pub decision: String,
}

impl PartialEq for Mulligan {
    fn eq(&self, other: &Self) -> bool {
        self.game_number == other.game_number && self.number_to_keep == other.number_to_keep
    }
}

impl Eq for Mulligan {}

impl PartialOrd for Mulligan {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Mulligan {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.game_number
            .cmp(&other.game_number)
            .then_with(|| self.number_to_keep.cmp(&other.number_to_keep).reverse())
    }
}

impl Mulligan {
    pub fn new(
        hand: &str,
        opponent_identity: String,
        game_number: i32,
        number_to_keep: i32,
        play_draw: String,
        decision: String,
        cards_database: &CardsDatabase,
    ) -> Self {
        let hand = hand
            .split(',')
            .filter_map(|card_id_str| card_id_str.parse::<i32>().ok())
            .map(|card_id| -> CardDisplayRecord {
                cards_database.get(&card_id).map_or_else(
                    || CardDisplayRecord::new(card_id.to_string()),
                    std::convert::Into::into,
                )
            })
            .collect();

        Self {
            hand,
            opponent_identity,
            game_number,
            number_to_keep,
            play_draw,
            decision,
        }
    }

    pub fn from_model(mulligan_info: &ModelMulligan, cards_database: &CardsDatabase) -> Self {
        Self::new(
            mulligan_info.hand(),
            mulligan_info.opponent_identity().to_string(),
            mulligan_info.game_number(),
            mulligan_info.number_to_keep(),
            mulligan_info.play_draw().to_string(),
            mulligan_info.decision().to_string(),
            cards_database,
        )
    }
}
