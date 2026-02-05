use super::{ArenaId, Deck, MTGAMatch, MatchResult, Mulligan};

/// Represents an opponent's deck in a match
///
/// This is the domain model for tracking which cards an opponent played,
/// separate from the wire format representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpponentDeck {
    pub cards: Vec<ArenaId>,
}

impl OpponentDeck {
    /// Creates a new opponent deck with the specified cards
    ///
    /// # Arguments
    ///
    /// * `cards` - A vector of `ArenaId` representing the cards in the opponent's deck
    ///
    /// # Returns
    ///
    /// A new `OpponentDeck` containing the specified cards
    pub fn new(cards: Vec<ArenaId>) -> Self {
        Self { cards }
    }

    /// Creates a new empty opponent deck
    ///
    /// # Returns
    ///
    /// A new `OpponentDeck` with no cards
    pub fn empty() -> Self {
        Self { cards: Vec::new() }
    }
}

/// Represents all data associated with a match
///
/// This is the domain model for a complete match, including the match metadata,
/// decks used, mulligan decisions, game results, and opponent's deck.
#[derive(Debug, Clone)]
pub struct MatchData {
    pub mtga_match: MTGAMatch,
    pub decks: Vec<Deck>,
    pub mulligans: Vec<Mulligan>,
    pub results: Vec<MatchResult>,
    pub opponent_deck: OpponentDeck,
}
