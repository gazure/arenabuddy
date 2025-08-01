use arenabuddy_core::models::{MTGAMatchBuilderError, MatchResultBuilderError};
use sqlx::types::uuid;

#[derive(Debug, thiserror::Error)]
pub enum MatchDBError {
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Db error: {0}")]
    PostgresError(#[from] sqlx::Error),
    #[error("MigrationError: {0}")]
    MigrationError(#[from] sqlx::migrate::MigrateError),
    #[error("EmbeddedDb error: {0}")]
    PostgresEmbeddedError(#[from] postgresql_embedded::Error),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("data error: {0}")]
    DataError(#[from] arenabuddy_core::Error),
    #[error("match result error: {0}")]
    MatchResultError(#[from] MatchResultBuilderError),
    #[error("match result error: {0}")]
    MatchError(#[from] MTGAMatchBuilderError),
    #[error("uuid parse error: {0}")]
    ParseError(#[from] uuid::Error),
}
