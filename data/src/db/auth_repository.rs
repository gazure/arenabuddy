use chrono::{DateTime, Utc};
use sqlx::types::Uuid;

use super::models::{AppUser, RefreshToken};
use crate::Result;

#[async_trait::async_trait]
pub trait AuthRepository: Send + Sync + 'static {
    async fn upsert_user(&self, discord_id: &str, username: &str, avatar_url: Option<&str>) -> Result<Uuid>;
    async fn get_user(&self, user_id: Uuid) -> Result<Option<AppUser>>;
    async fn create_refresh_token(&self, user_id: Uuid, token_hash: &[u8], expires_at: DateTime<Utc>) -> Result<()>;
    async fn find_refresh_token(&self, token_hash: &[u8]) -> Result<Option<RefreshToken>>;
    async fn revoke_refresh_token(&self, token_id: Uuid) -> Result<()>;
    async fn revoke_all_user_tokens(&self, user_id: Uuid) -> Result<()>;
    async fn find_token_owner(&self, token_hash: &[u8]) -> Result<Option<Uuid>>;
    async fn cleanup_expired_tokens(&self, user_id: Uuid) -> Result<()>;
}
