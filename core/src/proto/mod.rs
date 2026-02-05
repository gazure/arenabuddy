#![allow(clippy::all, clippy::pedantic)]

/// Proto-generated types and their conversions to domain models.
///
/// ## Module structure
///
/// The generated module hierarchy mirrors the proto package structure:
/// - `arenabuddy::models::v1` - Wire-format model types (Card, Deck, MtgaMatch, etc.)
/// - `arenabuddy::api::v1` - gRPC service types (Request/Response messages, service stubs)
///
/// ## Two patterns for using proto types
///
/// **Proto-as-domain** — The proto-generated struct *is* the domain type. Domain methods
/// and trait impls are added directly via `impl` blocks in the `models` module.
/// Use this when the proto fields map 1:1 to the domain with no type transformations.
/// Currently used for: `Card`, `CardFace`, `CardCollection`.
///
/// **Domain wrapper** — A separate Rust struct exists in `models` with its own fields,
/// and bidirectional `From` impls in the `convert` submodule bridge the two representations.
/// Use this when the domain type needs rich Rust types (e.g. `DateTime<Utc>`), external
/// context not present in the proto (e.g. `match_id`), builders, serde derives, or
/// significantly different field types (e.g. `ArenaId` vs `i32`).
/// Currently used for: `MTGAMatch`, `Deck`, `Mulligan`, `MatchResult`, `OpponentDeck`, `MatchData`.
pub mod arenabuddy {
    pub mod models {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/arenabuddy.models.v1.rs"));
        }
    }

    pub mod api {
        pub mod v1 {
            include!(concat!(env!("OUT_DIR"), "/arenabuddy.api.v1.rs"));
        }
    }
}

mod convert;

// Re-export model types at proto module level for convenience
pub use arenabuddy::models::v1::{
    Card, CardCollection, CardFace, Deck, MatchData, MatchResult, MtgaMatch, Mulligan, OpponentDeck,
};
