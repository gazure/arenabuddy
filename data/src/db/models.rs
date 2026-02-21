use sqlx::{FromRow, types::Uuid};

#[derive(Debug, FromRow)]
pub struct AppUser {
    pub id: Uuid,
    pub discord_id: String,
    pub username: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, FromRow)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub revoked: bool,
}
