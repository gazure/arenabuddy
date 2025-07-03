#[derive(Debug, thiserror::Error)]
pub enum MatchDBError {
    #[error("Unable to migrate: {0}")]
    MigrationError(rusqlite_migration::Error),
    #[error("sqlite error: {0}")]
    SqliteError(rusqlite::Error),
    #[error("serialization error: {0}")]
    SerializationError(serde_json::Error),
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

impl From<MatchDBError> for arenabuddy_core::Error {
    fn from(err: MatchDBError) -> Self {
        arenabuddy_core::Error::StorageError(err.to_string())
    }
}
