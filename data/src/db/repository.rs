use arenabuddy_core::{
    models::{Deck, Draft, MTGADraft, MTGAMatch, MatchResult, Mulligan},
    player_log::replay::MatchReplay,
};

use crate::Result;

pub trait ArenabuddyRepository: Send + Sync + 'static {
    fn init(&self) -> impl Future<Output = Result<()>> + Send;
    fn write_replay(&self, replay: &MatchReplay) -> impl Future<Output = Result<()>> + Send;
    fn list_matches(&self) -> impl Future<Output = Result<Vec<MTGAMatch>>> + Send;
    fn get_match(&self, match_id: &str) -> impl Future<Output = Result<(MTGAMatch, Option<MatchResult>)>> + Send;
    fn get_draft(&self, draft_id: &str) -> impl Future<Output = Result<MTGADraft>> + Send;
    fn get_opponent_deck(&self, match_id: &str) -> impl Future<Output = Result<Deck>> + Send;
    fn list_decklists(&self, match_id: &str) -> impl Future<Output = Result<Vec<Deck>>> + Send;
    fn list_mulligans(&self, match_id: &str) -> impl Future<Output = Result<Vec<Mulligan>>> + Send;
    fn list_match_results(&self, match_id: &str) -> impl Future<Output = Result<Vec<MatchResult>>> + Send;
    fn list_drafts(&self) -> impl Future<Output = Result<Vec<Draft>>> + Send;
}
