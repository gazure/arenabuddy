#![allow(clippy::all, clippy::pedantic)]

/// Root module for proto-generated types
///
/// The module structure mirrors the proto package structure:
/// - `arenabuddy::models::v1` - Domain model types (Card, Deck, MtgaMatch, etc.)
/// - `arenabuddy::api::v1` - Service types (Request/Response, gRPC service)
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

// Re-export model types at proto module level for convenience
pub use arenabuddy::models::v1::{
    Card, CardCollection, CardFace, Deck, MatchData, MatchResult, MtgaMatch, Mulligan, OpponentDeck,
};
