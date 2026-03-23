use super::metagame_models::{
    CardFrequencyRow, MatchArchetype, MetagameDeck, MetagameDeckCard, MetagameTournament, SignatureCard,
    SignatureCardRow, UnclassifiedMatchRow,
};
use crate::Result;

#[async_trait::async_trait]
pub trait MetagameRepository: Send + Sync + 'static {
    async fn upsert_metagame_tournament(&self, tournament: &MetagameTournament) -> Result<i32>;
    async fn upsert_metagame_archetype(&self, name: &str, format: &str, url: Option<&str>) -> Result<i32>;
    async fn upsert_metagame_deck(
        &self,
        deck: &MetagameDeck,
        tournament_id: Option<i32>,
        archetype_id: Option<i32>,
        cards: &[MetagameDeckCard],
    ) -> Result<i32>;
    async fn metagame_stats(&self, format: &str) -> Result<MetagameStatsResult>;

    // Classification methods
    async fn get_card_frequencies(&self, format: &str) -> Result<Vec<CardFrequencyRow>>;
    async fn replace_signature_cards(&self, format: &str, cards: &[SignatureCard]) -> Result<u64>;
    async fn get_signature_cards(&self, format: &str) -> Result<Vec<SignatureCardRow>>;
    async fn get_unclassified_matches(&self, format: &str) -> Result<Vec<UnclassifiedMatchRow>>;
    async fn upsert_match_archetype(&self, archetype: &MatchArchetype) -> Result<()>;
    async fn get_match_deck_cards(&self, match_id: &str) -> Result<Vec<String>>;
    async fn get_match_opponent_cards(&self, match_id: &str) -> Result<Vec<String>>;
    async fn get_match_archetypes(&self, match_id: &str) -> Result<(Option<String>, Option<String>)>;
}

#[derive(Debug, Clone)]
pub struct MetagameStatsResult {
    pub tournament_count: i64,
    pub archetype_count: i64,
    pub deck_count: i64,
    pub card_count: i64,
}
