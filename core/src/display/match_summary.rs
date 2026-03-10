use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchSummary {
    pub id: String,
    pub controller_player_name: String,
    pub opponent_player_name: String,
    pub created_at: DateTime<Utc>,
    pub did_controller_win: Option<bool>,
    pub game_wins: i64,
    pub game_losses: i64,
}

impl MatchSummary {
    pub fn game_score(&self) -> String {
        format!("{}-{}", self.game_wins, self.game_losses)
    }
}
