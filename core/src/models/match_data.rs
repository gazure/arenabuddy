use super::{ArenaId, Deck, MTGAMatch, MatchResult, Mulligan};
use crate::proto;

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

// Conversions between domain models and proto types

impl From<&proto::OpponentDeck> for OpponentDeck {
    fn from(proto: &proto::OpponentDeck) -> Self {
        Self {
            cards: proto.cards.iter().map(|&id| ArenaId::from(id)).collect(),
        }
    }
}

impl From<&OpponentDeck> for proto::OpponentDeck {
    fn from(deck: &OpponentDeck) -> Self {
        Self {
            cards: deck.cards.iter().map(ArenaId::inner).collect(),
        }
    }
}

impl From<&proto::MatchData> for MatchData {
    fn from(proto: &proto::MatchData) -> Self {
        let mtga_match_proto = proto.mtga_match.as_ref().expect("MatchData must have mtga_match");
        let mtga_match = MTGAMatch::from(mtga_match_proto);
        let match_id = mtga_match.id().to_string();

        Self {
            mtga_match,
            decks: proto.decks.iter().map(Deck::from).collect(),
            mulligans: proto
                .mulligans
                .iter()
                .map(|m| Mulligan::from((match_id.as_str(), m)))
                .collect(),
            results: proto
                .results
                .iter()
                .map(|r| MatchResult::from((match_id.as_str(), r)))
                .collect(),
            opponent_deck: proto
                .opponent_deck
                .as_ref()
                .map_or_else(OpponentDeck::empty, OpponentDeck::from),
        }
    }
}

impl From<&MatchData> for proto::MatchData {
    fn from(data: &MatchData) -> Self {
        Self {
            mtga_match: Some((&data.mtga_match).into()),
            decks: data.decks.iter().map(std::convert::Into::into).collect(),
            mulligans: data.mulligans.iter().map(std::convert::Into::into).collect(),
            results: data.results.iter().map(std::convert::Into::into).collect(),
            opponent_deck: Some((&data.opponent_deck).into()),
        }
    }
}
