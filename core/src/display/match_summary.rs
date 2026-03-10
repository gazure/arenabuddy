use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchSummary {
    pub id: String,
    pub controller_player_name: String,
    pub opponent_player_name: String,
    pub created_at: DateTime<Utc>,
    pub format: Option<String>,
    pub did_controller_win: Option<bool>,
    pub game_wins: i64,
    pub game_losses: i64,
}

impl MatchSummary {
    pub fn game_score(&self) -> String {
        format!("{}-{}", self.game_wins, self.game_losses)
    }

    pub fn display_format(&self) -> &str {
        self.format.as_deref().map_or("Unknown", format_event_id)
    }
}

/// Converts a raw MTGA `event_id` like `"Traditional_Ladder"` into `"Traditional Ladder"`.
pub fn format_event_id(event_id: &str) -> &str {
    match event_id {
        "Traditional_Ladder" => "Traditional Standard",
        "Ladder" => "Ranked Standard",
        "Traditional_Explorer_Ladder" => "Traditional Explorer",
        "Explorer_Ladder" => "Ranked Explorer",
        "Traditional_Historic_Ladder" => "Traditional Historic",
        "Historic_Ladder" => "Ranked Historic",
        "Traditional_Timeless_Ladder" => "Traditional Timeless",
        "Timeless_Ladder" => "Ranked Timeless",
        _ => event_id,
    }
}
