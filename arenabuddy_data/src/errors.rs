use arenabuddy_core::models::{MTGAMatchBuilderError, MatchResultBuilderError};

#[derive(Debug, thiserror::Error)]
pub enum MatchDBError {
    #[error("Io error: {0}")]
    IoError(#[from] std::io::Error),
    #[error("Unable to migrate: {0}")]
    MigrationError(#[from] rusqlite_migration::Error),
    #[error("sqlite error: {0}")]
    SqliteError(#[from] rusqlite::Error),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("data error: {0}")]
    DataError(#[from] arenabuddy_core::Error),
    #[error("match result error: {0}")]
    MatchResultError(#[from] MatchResultBuilderError),
    #[error("match result error: {0}")]
    MatchError(#[from] MTGAMatchBuilderError),
}
