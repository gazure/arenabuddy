#![expect(clippy::cast_precision_loss)]

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
