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
    pub controller_archetype: Option<String>,
    pub opponent_archetype: Option<String>,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn format_event_id_known_formats() {
        assert_eq!(format_event_id("Traditional_Ladder"), "Traditional Standard");
        assert_eq!(format_event_id("Ladder"), "Ranked Standard");
        assert_eq!(format_event_id("Traditional_Explorer_Ladder"), "Traditional Explorer");
        assert_eq!(format_event_id("Explorer_Ladder"), "Ranked Explorer");
        assert_eq!(format_event_id("Traditional_Historic_Ladder"), "Traditional Historic");
        assert_eq!(format_event_id("Historic_Ladder"), "Ranked Historic");
        assert_eq!(format_event_id("Traditional_Timeless_Ladder"), "Traditional Timeless");
        assert_eq!(format_event_id("Timeless_Ladder"), "Ranked Timeless");
    }

    #[test]
    fn format_event_id_unknown_passes_through() {
        assert_eq!(format_event_id("SomeNewFormat_2025"), "SomeNewFormat_2025");
        assert_eq!(format_event_id(""), "");
    }

    #[test]
    fn game_score_formatting() {
        let summary = MatchSummary {
            game_wins: 2,
            game_losses: 1,
            ..Default::default()
        };
        assert_eq!(summary.game_score(), "2-1");
    }

    #[test]
    fn game_score_zero() {
        let summary = MatchSummary::default();
        assert_eq!(summary.game_score(), "0-0");
    }

    #[test]
    fn display_format_uses_format_event_id() {
        let summary = MatchSummary {
            format: Some("Traditional_Ladder".to_string()),
            ..Default::default()
        };
        assert_eq!(summary.display_format(), "Traditional Standard");
    }

    #[test]
    fn display_format_none_shows_unknown() {
        let summary = MatchSummary::default();
        assert_eq!(summary.display_format(), "Unknown");
    }
}
