use chrono::Utc;
use serde::{Deserialize, Serialize};

use super::{ArenaId, Deck, GameEventLog, MTGAMatch, MTGAMatchBuilder, MatchResult, Mulligan};
use crate::{cards::CardsDatabase, player_log::replay::MatchReplay};

/// Represents an opponent's deck in a match
///
/// This is the domain model for tracking which cards an opponent played,
/// separate from the wire format representation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MatchData {
    pub mtga_match: MTGAMatch,
    pub decks: Vec<Deck>,
    pub mulligans: Vec<Mulligan>,
    pub results: Vec<MatchResult>,
    pub opponent_deck: OpponentDeck,
    pub event_logs: Vec<GameEventLog>,
}

impl MatchData {
    /// Builds the data to persist and upload from a replay and its card database.
    ///
    /// # Errors
    ///
    /// Returns an error if required replay fields cannot be extracted.
    pub fn from_replay(replay: &MatchReplay, cards: &CardsDatabase) -> crate::Result<Self> {
        let seat = replay.get_controller_seat_id();
        let (controller, opponent) = replay.get_player_names(seat)?;
        let mtga_match = MTGAMatchBuilder::default()
            .id(replay.match_id.clone())
            .controller_seat_id(seat)
            .controller_player_name(controller)
            .opponent_player_name(opponent)
            .created_at(replay.match_start_time().unwrap_or_else(Utc::now))
            .format(replay.match_format())
            .build()?;
        let results = replay
            .get_match_results()?
            .result_list
            .iter()
            .enumerate()
            .map(|(i, result)| {
                let game_number = if result.scope == "MatchScope_Game" {
                    i32::try_from(i + 1).unwrap_or(0)
                } else {
                    0
                };
                MatchResult::new(&replay.match_id, game_number, result.winning_team_id, &result.scope)
            })
            .collect();
        Ok(Self {
            mtga_match,
            decks: replay.get_decklists()?,
            mulligans: replay.get_mulligan_infos(cards)?,
            results,
            opponent_deck: OpponentDeck::new(replay.get_opponent_cards()),
            event_logs: replay.get_event_logs(cards),
        })
    }
}
