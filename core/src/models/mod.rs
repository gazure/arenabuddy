//! Domain model types for arenabuddy.
//!
//! This module exports only domain types — proto/wire-format types are not re-exported here.
//! Some types (e.g. `Card`) are the proto-generated struct directly with domain methods added;
//! others (e.g. `MTGAMatch`) are separate Rust structs with conversions defined in
//! `crate::proto::convert`. See the `proto` module docs for the full pattern description.

mod card;
mod deck;
mod draft;
mod id;
mod mana;
mod match_data;
mod match_result;
mod mtga_match;
mod mulligan;

pub use card::{Card, CardCollection, CardFace, CardType};
pub use deck::{Deck, Quantities};
pub use draft::{Draft, DraftPack, Format, MTGADraft};
pub use id::ArenaId;
pub use mana::{Color, Cost, CostSymbol};
pub use match_data::{MatchData, OpponentDeck};
pub use match_result::{MatchResult, MatchResultBuilder, MatchResultBuilderError};
pub use mtga_match::{MTGAMatch, MTGAMatchBuilder, MTGAMatchBuilderError};
pub use mulligan::{Mulligan, MulliganBuilder};
