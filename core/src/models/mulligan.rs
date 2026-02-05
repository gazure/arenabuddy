use derive_builder::Builder;
use serde::{Deserialize, Serialize};

/// Represents a mulligan decision in a Magic: The Gathering Arena game
///
/// A mulligan contains information about a player's hand and their decision
/// to keep or mulligan the hand.
#[derive(Debug, Clone, Builder, Serialize, Deserialize)]
#[builder(setter(into))]
pub struct Mulligan {
    match_id: String,
    game_number: i32,
    number_to_keep: i32,
    hand: String,
    play_draw: String,
    opponent_identity: String,
    decision: String,
}

impl Mulligan {
    /// Creates a new mulligan record
    ///
    /// # Arguments
    ///
    /// * `match_id` - The ID of the match this mulligan belongs to
    /// * `game_number` - The game number within the match
    /// * `number_to_keep` - The number of cards to keep after mulliganing
    /// * `hand` - A string representation of the cards in the hand
    /// * `play_draw` - Whether the player is on the play or draw
    /// * `opponent_identity` - The identity or deck archetype of the opponent
    /// * `decision` - The decision made (e.g., "keep" or "mulligan")
    ///
    /// # Returns
    ///
    /// A new Mulligan instance
    pub fn new(
        match_id: impl Into<String>,
        game_number: i32,
        number_to_keep: i32,
        hand: impl Into<String>,
        play_draw: impl Into<String>,
        opponent_identity: impl Into<String>,
        decision: impl Into<String>,
    ) -> Self {
        Self {
            match_id: match_id.into(),
            game_number,
            number_to_keep,
            hand: hand.into(),
            play_draw: play_draw.into(),
            opponent_identity: opponent_identity.into(),
            decision: decision.into(),
        }
    }

    /// Returns the match ID
    pub fn match_id(&self) -> &str {
        &self.match_id
    }

    /// Returns the game number
    pub fn game_number(&self) -> i32 {
        self.game_number
    }

    /// Returns the number of cards to keep after mulliganing
    pub fn number_to_keep(&self) -> i32 {
        self.number_to_keep
    }

    /// Returns the hand string
    pub fn hand(&self) -> &str {
        &self.hand
    }

    /// Returns the play/draw status
    pub fn play_draw(&self) -> &str {
        &self.play_draw
    }

    /// Returns the opponent identity
    pub fn opponent_identity(&self) -> &str {
        &self.opponent_identity
    }

    /// Returns the decision made
    pub fn decision(&self) -> &str {
        &self.decision
    }

    /// Returns whether the player decided to keep their hand
    pub fn did_keep(&self) -> bool {
        self.decision.to_lowercase() == "keep"
    }

    /// Returns whether the player decided to mulligan their hand
    pub fn did_mulligan(&self) -> bool {
        self.decision.to_lowercase() == "mulligan"
    }

    /// Returns whether the player is on the play
    pub fn is_on_play(&self) -> bool {
        self.play_draw.to_lowercase() == "play"
    }

    /// Returns whether the player is on the draw
    pub fn is_on_draw(&self) -> bool {
        self.play_draw.to_lowercase() == "draw"
    }

    /// Returns the number of cards in the initial hand
    ///
    /// This parses the hand string which should be a JSON array of card IDs
    pub fn initial_hand_size(&self) -> usize {
        match serde_json::from_str::<Vec<i32>>(&self.hand) {
            Ok(cards) => cards.len(),
            Err(_) => 0,
        }
    }

    /// Returns the cards in the hand as a vector of card IDs
    ///
    /// # Returns
    ///
    /// A vector of card IDs, or an empty vector if the hand string couldn't be parsed
    pub fn hand_cards(&self) -> Vec<i32> {
        serde_json::from_str(&self.hand).unwrap_or_default()
    }
}
