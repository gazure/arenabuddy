use arenabuddy_core::{
    models::{ArenaId, Deck, Draft, MTGADraft, MTGAMatch, MatchResult, Mulligan},
    player_log::replay::MatchReplay,
};
use sqlx::types::Uuid;

use crate::Result;

#[async_trait::async_trait]
pub trait ArenabuddyRepository: Send + Sync + 'static {
    async fn init(&self) -> Result<()>;
    async fn write_replay(&self, replay: &MatchReplay) -> Result<()>;
    async fn list_matches(&self, user_id: Option<Uuid>) -> Result<Vec<MTGAMatch>>;
    async fn get_match(&self, match_id: &str, user_id: Option<Uuid>) -> Result<(MTGAMatch, Option<MatchResult>)>;
    async fn get_draft(&self, draft_id: &str) -> Result<MTGADraft>;
    async fn get_opponent_deck(&self, match_id: &str) -> Result<Deck>;
    async fn list_decklists(&self, match_id: &str) -> Result<Vec<Deck>>;
    async fn list_mulligans(&self, match_id: &str) -> Result<Vec<Mulligan>>;
    async fn list_match_results(&self, match_id: &str) -> Result<Vec<MatchResult>>;
    async fn list_drafts(&self) -> Result<Vec<Draft>>;

    async fn upsert_match_data(
        &self,
        mtga_match: &MTGAMatch,
        decks: &[Deck],
        mulligans: &[Mulligan],
        results: &[MatchResult],
        opponent_cards: &[ArenaId],
        user_id: Option<Uuid>,
    ) -> Result<()>;

    async fn delete_match(&self, match_id: &str, user_id: Option<Uuid>) -> Result<()>;
}
