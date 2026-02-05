//! Bidirectional conversions between proto-generated types and domain models.
//!
//! This module centralizes all `From` implementations that bridge the wire format
//! (proto types in `crate::proto`) and the domain model types (in `crate::models`).
//!
//! ## When to add conversions here
//!
//! Any type that uses the **domain wrapper** pattern (i.e., has a separate Rust struct
//! distinct from the proto-generated struct) needs `From` impls in this module.
//! Types using the **proto-as-domain** pattern (e.g., `Card`, `CardFace`, `CardCollection`)
//! do not need conversions — the proto type *is* the domain type.

use chrono::{DateTime, Utc};

use super::{
    MatchData as MatchDataProto, MatchResult as MatchResultProto, MtgaMatch as MtgaMatchProto,
    Mulligan as MulliganProto, OpponentDeck as OpponentDeckProto,
};
use crate::models::{ArenaId, MTGAMatch, MatchData as MatchDataDomain, MatchResult, Mulligan, OpponentDeck};

// --- MTGAMatch ↔ MtgaMatch proto ---

impl From<&MtgaMatchProto> for MTGAMatch {
    fn from(proto: &MtgaMatchProto) -> Self {
        let created_at = DateTime::parse_from_rfc3339(&proto.created_at)
            .map(|dt| dt.with_timezone(&Utc))
            .unwrap_or_default();
        Self::new_with_timestamp(
            &proto.id,
            proto.controller_seat_id,
            &proto.controller_player_name,
            &proto.opponent_player_name,
            created_at,
        )
    }
}

impl From<&MTGAMatch> for MtgaMatchProto {
    fn from(m: &MTGAMatch) -> Self {
        Self {
            id: m.id().to_string(),
            controller_seat_id: m.controller_seat_id(),
            controller_player_name: m.controller_player_name().to_string(),
            opponent_player_name: m.opponent_player_name().to_string(),
            created_at: m.created_at().to_rfc3339(),
        }
    }
}

// --- Mulligan ↔ Mulligan proto ---
//
// Note: The domain Mulligan carries a `match_id` that the proto does not.
// When converting proto → domain, the match_id must be supplied externally.

impl From<(&str, &MulliganProto)> for Mulligan {
    fn from((match_id, proto): (&str, &MulliganProto)) -> Self {
        Self::new(
            match_id,
            proto.game_number,
            proto.number_to_keep,
            &proto.hand,
            &proto.play_draw,
            &proto.opponent_identity,
            &proto.decision,
        )
    }
}

impl From<&Mulligan> for MulliganProto {
    fn from(m: &Mulligan) -> Self {
        Self {
            game_number: m.game_number(),
            number_to_keep: m.number_to_keep(),
            hand: m.hand().to_string(),
            play_draw: m.play_draw().to_string(),
            opponent_identity: m.opponent_identity().to_string(),
            decision: m.decision().to_string(),
        }
    }
}

// --- MatchResult ↔ MatchResult proto ---
//
// Note: Same as Mulligan — the domain MatchResult carries a `match_id` not in the proto.

impl From<(&str, &MatchResultProto)> for MatchResult {
    fn from((match_id, proto): (&str, &MatchResultProto)) -> Self {
        Self::new(match_id, proto.game_number, proto.winning_team_id, &proto.result_scope)
    }
}

impl From<&MatchResult> for MatchResultProto {
    fn from(r: &MatchResult) -> Self {
        Self {
            game_number: r.game_number(),
            winning_team_id: r.winning_team_id(),
            result_scope: r.result_scope().to_string(),
        }
    }
}

// --- OpponentDeck ↔ OpponentDeck proto ---

impl From<&OpponentDeckProto> for OpponentDeck {
    fn from(proto: &OpponentDeckProto) -> Self {
        Self::new(proto.cards.iter().map(|&id| ArenaId::from(id)).collect())
    }
}

impl From<&OpponentDeck> for OpponentDeckProto {
    fn from(deck: &OpponentDeck) -> Self {
        Self {
            cards: deck.cards.iter().map(ArenaId::inner).collect(),
        }
    }
}

// --- MatchData ↔ MatchData proto ---

impl From<&MatchDataProto> for MatchDataDomain {
    fn from(proto: &MatchDataProto) -> Self {
        let mtga_match_proto = proto.mtga_match.as_ref().expect("MatchData must have mtga_match");
        let mtga_match = MTGAMatch::from(mtga_match_proto);
        let match_id = mtga_match.id().to_string();

        Self {
            mtga_match,
            decks: proto.decks.clone(),
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

impl From<&MatchDataDomain> for MatchDataProto {
    fn from(data: &MatchDataDomain) -> Self {
        Self {
            mtga_match: Some(MtgaMatchProto::from(&data.mtga_match)),
            decks: data.decks.clone(),
            mulligans: data.mulligans.iter().map(MulliganProto::from).collect(),
            results: data.results.iter().map(MatchResultProto::from).collect(),
            opponent_deck: Some(OpponentDeckProto::from(&data.opponent_deck)),
        }
    }
}
