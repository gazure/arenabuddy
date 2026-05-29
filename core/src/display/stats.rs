#![expect(clippy::cast_precision_loss)]

use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TimeWindow {
    Last24Hours,
    Last7Days,
    Last30Days,
    #[default]
    AllTime,
}

impl TimeWindow {
    pub const ALL: [TimeWindow; 4] = [
        TimeWindow::Last24Hours,
        TimeWindow::Last7Days,
        TimeWindow::Last30Days,
        TimeWindow::AllTime,
    ];

    pub fn label(self) -> &'static str {
        match self {
            TimeWindow::Last24Hours => "Last 24 Hours",
            TimeWindow::Last7Days => "Last 7 Days",
            TimeWindow::Last30Days => "Last 30 Days",
            TimeWindow::AllTime => "All Time",
        }
    }

    pub fn cutoff(self) -> Option<DateTime<Utc>> {
        let now = Utc::now();
        match self {
            TimeWindow::Last24Hours => Some(now - chrono::Duration::hours(24)),
            TimeWindow::Last7Days => Some(now - chrono::Duration::days(7)),
            TimeWindow::Last30Days => Some(now - chrono::Duration::days(30)),
            TimeWindow::AllTime => None,
        }
    }
}

impl std::fmt::Display for TimeWindow {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.label())
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MatchStats {
    pub total_matches: i64,
    pub match_wins: i64,
    pub match_losses: i64,
    pub total_games: i64,
    pub game_wins: i64,
    pub game_losses: i64,
    pub play_wins: i64,
    pub play_losses: i64,
    pub draw_wins: i64,
    pub draw_losses: i64,
    pub mulligan_stats: Vec<MulliganBucket>,
    pub opponents: Vec<OpponentRecord>,
}

impl MatchStats {
    pub fn match_win_rate(&self) -> Option<f64> {
        let total = self.match_wins + self.match_losses;
        (total > 0).then(|| self.match_wins as f64 / total as f64 * 100.0)
    }

    pub fn game_win_rate(&self) -> Option<f64> {
        let total = self.game_wins + self.game_losses;
        (total > 0).then(|| self.game_wins as f64 / total as f64 * 100.0)
    }

    pub fn play_win_rate(&self) -> Option<f64> {
        let total = self.play_wins + self.play_losses;
        (total > 0).then(|| self.play_wins as f64 / total as f64 * 100.0)
    }

    pub fn draw_win_rate(&self) -> Option<f64> {
        let total = self.draw_wins + self.draw_losses;
        (total > 0).then(|| self.draw_wins as f64 / total as f64 * 100.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct MulliganBucket {
    pub cards_kept: i32,
    pub count: i64,
    pub wins: i64,
    pub losses: i64,
}

impl MulliganBucket {
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.wins + self.losses;
        (total > 0).then(|| self.wins as f64 / total as f64 * 100.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct OpponentRecord {
    pub name: String,
    pub matches: i64,
    pub wins: i64,
    pub losses: i64,
}

impl OpponentRecord {
    pub fn win_rate(&self) -> Option<f64> {
        let total = self.wins + self.losses;
        (total > 0).then(|| self.wins as f64 / total as f64 * 100.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- MatchStats win rates -------------------------------------------------

    #[test]
    fn match_win_rate_basic() {
        let stats = MatchStats {
            match_wins: 7,
            match_losses: 3,
            ..Default::default()
        };
        let rate = stats.match_win_rate().expect("should have rate");
        assert!((rate - 70.0).abs() < f64::EPSILON);
    }

    #[test]
    fn match_win_rate_zero_games_returns_none() {
        let stats = MatchStats::default();
        assert!(stats.match_win_rate().is_none());
    }

    #[test]
    fn game_win_rate_basic() {
        let stats = MatchStats {
            game_wins: 3,
            game_losses: 1,
            ..Default::default()
        };
        let rate = stats.game_win_rate().expect("should have rate");
        assert!((rate - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn game_win_rate_zero_returns_none() {
        let stats = MatchStats::default();
        assert!(stats.game_win_rate().is_none());
    }

    #[test]
    fn play_win_rate_basic() {
        let stats = MatchStats {
            play_wins: 5,
            play_losses: 5,
            ..Default::default()
        };
        let rate = stats.play_win_rate().expect("should have rate");
        assert!((rate - 50.0).abs() < f64::EPSILON);
    }

    #[test]
    fn play_win_rate_zero_returns_none() {
        let stats = MatchStats::default();
        assert!(stats.play_win_rate().is_none());
    }

    #[test]
    fn draw_win_rate_basic() {
        let stats = MatchStats {
            draw_wins: 1,
            draw_losses: 3,
            ..Default::default()
        };
        let rate = stats.draw_win_rate().expect("should have rate");
        assert!((rate - 25.0).abs() < f64::EPSILON);
    }

    #[test]
    fn draw_win_rate_zero_returns_none() {
        let stats = MatchStats::default();
        assert!(stats.draw_win_rate().is_none());
    }

    #[test]
    fn win_rate_all_wins() {
        let stats = MatchStats {
            match_wins: 10,
            match_losses: 0,
            ..Default::default()
        };
        let rate = stats.match_win_rate().expect("should have rate");
        assert!((rate - 100.0).abs() < f64::EPSILON);
    }

    #[test]
    fn win_rate_all_losses() {
        let stats = MatchStats {
            match_wins: 0,
            match_losses: 10,
            ..Default::default()
        };
        let rate = stats.match_win_rate().expect("should have rate");
        assert!(rate.abs() < f64::EPSILON);
    }

    // -- MulliganBucket -------------------------------------------------------

    #[test]
    fn mulligan_bucket_win_rate() {
        let bucket = MulliganBucket {
            cards_kept: 7,
            count: 10,
            wins: 6,
            losses: 4,
        };
        let rate = bucket.win_rate().expect("should have rate");
        assert!((rate - 60.0).abs() < f64::EPSILON);
    }

    #[test]
    fn mulligan_bucket_zero_games() {
        let bucket = MulliganBucket::default();
        assert!(bucket.win_rate().is_none());
    }

    // -- OpponentRecord -------------------------------------------------------

    #[test]
    fn opponent_record_win_rate() {
        let record = OpponentRecord {
            name: "Opp".to_string(),
            matches: 4,
            wins: 3,
            losses: 1,
        };
        let rate = record.win_rate().expect("should have rate");
        assert!((rate - 75.0).abs() < f64::EPSILON);
    }

    #[test]
    fn opponent_record_zero_games() {
        let record = OpponentRecord::default();
        assert!(record.win_rate().is_none());
    }

    // -- TimeWindow -----------------------------------------------------------

    #[test]
    fn time_window_all_time_cutoff_is_none() {
        assert!(TimeWindow::AllTime.cutoff().is_none());
    }

    #[test]
    fn time_window_last_24h_cutoff_is_some() {
        let cutoff = TimeWindow::Last24Hours.cutoff().expect("should have cutoff");
        let now = Utc::now();
        let diff = now - cutoff;
        assert!((diff.num_hours() - 24).abs() <= 1);
    }

    #[test]
    fn time_window_labels() {
        assert_eq!(TimeWindow::Last24Hours.label(), "Last 24 Hours");
        assert_eq!(TimeWindow::Last7Days.label(), "Last 7 Days");
        assert_eq!(TimeWindow::Last30Days.label(), "Last 30 Days");
        assert_eq!(TimeWindow::AllTime.label(), "All Time");
    }
}
