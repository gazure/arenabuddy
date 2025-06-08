use serde::{Deserialize, Serialize};

use crate::models::MatchResult;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct GameResultDisplay {
    pub game_number: i32,
    pub winning_player: String,
}

impl GameResultDisplay {
    pub fn from_match_result(
        mr: &MatchResult,
        controller_seat_id: i32,
        controller_player_name: &str,
        opponent_player_name: &str,
    ) -> Self {
        Self {
            game_number: mr.game_number(),
            winning_player: if mr.winning_team_id() == controller_seat_id {
                controller_player_name.into()
            } else {
                opponent_player_name.into()
            },
        }
    }
}
