use serde::{Deserialize, Serialize};

use crate::{
    cards::CardsDatabase, display::card::CardDisplayRecord, models::mulligan::MulliganInfo,
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

    pub fn from_mulligan_info(
        mulligan_info: &MulliganInfo,
        cards_database: &CardsDatabase,
    ) -> Self {
        Self::new(
            &mulligan_info.hand,
            mulligan_info.opponent_identity.clone(),
            mulligan_info.game_number,
            mulligan_info.number_to_keep,
            mulligan_info.play_draw.clone(),
            mulligan_info.decision.clone(),
            cards_database,
        )
    }
}
