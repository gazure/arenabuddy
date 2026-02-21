use chrono::{DateTime, Utc};
use sqlx::types::Uuid;

use crate::Result;

#[async_trait::async_trait]
pub trait DebugRepository: Send + Sync + 'static {
    async fn insert_parse_error(&self, user_id: Option<Uuid>, raw_json: &str, reported_at: DateTime<Utc>)
    -> Result<()>;
}
