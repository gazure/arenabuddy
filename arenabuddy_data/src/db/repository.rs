use arenabuddy_core::{
    models::{Deck, Draft, MTGAMatch, MatchResult, Mulligan},
    replay::MatchReplay,
};

use crate::{ReplayStorage, Result};

pub trait ArenabuddyRepository: Send + Sync + 'static {
    fn init(&mut self) -> impl Future<Output = Result<()>> + Send;
    fn write_replay(&mut self, replay: &MatchReplay) -> impl Future<Output = Result<()>> + Send;
    fn list_matches(&mut self) -> impl Future<Output = Result<Vec<MTGAMatch>>> + Send;
    fn get_match(&mut self, match_id: &str) -> impl Future<Output = Result<(MTGAMatch, Option<MatchResult>)>> + Send;
    fn get_opponent_deck(&mut self, match_id: &str) -> impl Future<Output = Result<Deck>> + Send;
    fn list_decklists(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<Deck>>> + Send;
    fn list_mulligans(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<Mulligan>>> + Send;
    fn list_match_results(&mut self, match_id: &str) -> impl Future<Output = Result<Vec<MatchResult>>> + Send;
    fn list_drafts(&mut self) -> impl Future<Output = Result<Vec<Draft>>> + Send;
}

impl<AR> ReplayStorage for AR
where
    AR: ArenabuddyRepository,
{
    async fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()> {
        self.write_replay(match_replay).await
    }
}
