use arenabuddy_core::models::Card;

use crate::Result;

#[async_trait::async_trait]
pub trait CardRepository: Send + Sync {
    async fn load_cards(&self, cards: &[Card]) -> Result<()>;
    async fn get_card(&self, arena_id: i64) -> Result<Option<Card>>;
    async fn get_cards(&self, arena_ids: &[i64]) -> Result<Vec<Card>>;
    async fn card_count(&self) -> Result<i64>;
}
