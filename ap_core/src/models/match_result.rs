use derive_builder::Builder;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
pub struct MatchResult {
    pub match_id: String,
    pub game_number: i32,
    pub winning_team_id: i32,
    pub result_scope: String,
}
