use arenabuddy_core::models::{MTGAMatchBuilderError, MatchResultBuilderError};

#[derive(Debug, thiserror::Error)]
pub enum MatchDBError {
    #[error("Io error: {0}")]
    IoError(std::io::Error),
    #[error("Unable to migrate: {0}")]
    MigrationError(rusqlite_migration::Error),
    #[error("sqlite error: {0}")]
    SqliteError(rusqlite::Error),
    #[error("serialization error: {0}")]
    SerializationError(serde_json::Error),
    #[error("data error: {0}")]
    DataError(arenabuddy_core::Error),
    #[error("match result error: {0}")]
    MatchResultError(MatchResultBuilderError),
    #[error("match result error: {0}")]
    MatchError(MTGAMatchBuilderError),
}

impl From<rusqlite_migration::Error> for MatchDBError {
    fn from(err: rusqlite_migration::Error) -> Self {
        MatchDBError::MigrationError(err)
    }
}

impl From<rusqlite::Error> for MatchDBError {
    fn from(err: rusqlite::Error) -> Self {
        MatchDBError::SqliteError(err)
    }
}

impl From<serde_json::Error> for MatchDBError {
    fn from(err: serde_json::Error) -> Self {
        MatchDBError::SerializationError(err)
    }
}

impl From<arenabuddy_core::Error> for MatchDBError {
    fn from(err: arenabuddy_core::Error) -> Self {
        Self::DataError(err)
    }
}

impl From<std::io::Error> for MatchDBError {
    fn from(err: std::io::Error) -> Self {
        Self::IoError(err)
    }
}

impl From<MatchResultBuilderError> for MatchDBError {
    fn from(err: MatchResultBuilderError) -> Self {
        Self::MatchResultError(err)
    }
}

impl From<MTGAMatchBuilderError> for MatchDBError {
    fn from(err: MTGAMatchBuilderError) -> Self {
        Self::MatchError(err)
    }
}
