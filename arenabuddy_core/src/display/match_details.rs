use crate::display::deck::{DeckDifference, DeckDisplayRecord};
use crate::display::game::GameResultDisplay;
use crate::display::mulligan::Mulligan;
use crate::models::deck::Deck;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct MatchDetails {
    pub id: String,
    pub did_controller_win: bool,
    pub controller_seat_id: i32,
    pub controller_player_name: String,
    pub opponent_player_name: String,
    pub created_at: DateTime<Utc>,
    pub primary_decklist: Option<DeckDisplayRecord>,
    pub differences: Option<Vec<DeckDifference>>,
    pub game_results: Vec<GameResultDisplay>,
    pub decklists: Vec<Deck>,
    pub mulligans: Vec<Mulligan>,
}
