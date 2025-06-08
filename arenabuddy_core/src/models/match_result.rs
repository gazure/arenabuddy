use derive_builder::Builder;
use serde::{Deserialize, Serialize};

/// Represents the result of a Magic: The Gathering Arena match
///
/// A match result contains information about which player/team won a specific game
/// and the scope of the result (e.g., game, match).
#[derive(Debug, Clone, Serialize, Deserialize, Builder)]
#[builder(setter(into))]
pub struct MatchResult {
    match_id: String,
    game_number: i32,
    winning_team_id: i32,
    result_scope: String,
}

impl MatchResult {
    /// Creates a new match result
    ///
    /// # Arguments
    ///
    /// * `match_id` - The ID of the match this result belongs to
    /// * `game_number` - The game number within the match (typically 1, 2, or 3)
    /// * `winning_team_id` - The ID of the team that won the game
    /// * `result_scope` - The scope of the result (e.g., "Game", "Match")
    ///
    /// # Returns
    ///
    /// A new MatchResult instance
    pub fn new(
        match_id: impl Into<String>,
        game_number: i32,
        winning_team_id: i32,
        result_scope: impl Into<String>,
    ) -> Self {
        Self {
            match_id: match_id.into(),
            game_number,
            winning_team_id,
            result_scope: result_scope.into(),
        }
    }

    /// Creates a new game result
    ///
    /// # Arguments
    ///
    /// * `match_id` - The ID of the match this result belongs to
    /// * `game_number` - The game number within the match
    /// * `winning_team_id` - The ID of the team that won the game
    ///
    /// # Returns
    ///
    /// A new MatchResult instance with "Game" scope
    pub fn new_game_result(
        match_id: impl Into<String>,
        game_number: i32,
        winning_team_id: i32,
    ) -> Self {
        Self::new(match_id, game_number, winning_team_id, "Game")
    }

    /// Creates a new match result
    ///
    /// # Arguments
    ///
    /// * `match_id` - The ID of the match this result belongs to
    /// * `winning_team_id` - The ID of the team that won the match
    ///
    /// # Returns
    ///
    /// A new MatchResult instance with "Match" scope and game_number 0
    pub fn new_match_result(
        match_id: impl Into<String>,
        winning_team_id: i32,
    ) -> Self {
        Self::new(match_id, 0, winning_team_id, "Match")
    }

    /// Returns the match ID
    pub fn match_id(&self) -> &str {
        &self.match_id
    }

    /// Returns the game number
    pub fn game_number(&self) -> i32 {
        self.game_number
    }

    /// Returns the winning team ID
    pub fn winning_team_id(&self) -> i32 {
        self.winning_team_id
    }

    /// Returns the result scope
    pub fn result_scope(&self) -> &str {
        &self.result_scope
    }

    /// Checks if this result is for a specific game
    ///
    /// # Returns
    ///
    /// true if the result scope is "Game", false otherwise
    pub fn is_game_result(&self) -> bool {
        self.result_scope == "Game"
    }

    /// Checks if this result is for the entire match
    ///
    /// # Returns
    ///
    /// true if the result scope is "Match", false otherwise
    pub fn is_match_result(&self) -> bool {
        self.result_scope == "Match"
    }

    /// Checks if the specified team ID is the winner
    ///
    /// # Arguments
    ///
    /// * `team_id` - The team ID to check
    ///
    /// # Returns
    ///
    /// true if the specified team ID is the winner, false otherwise
    pub fn is_winner(&self, team_id: i32) -> bool {
        self.winning_team_id == team_id
    }
}
