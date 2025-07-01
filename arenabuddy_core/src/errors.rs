use crate::{
    models::{MTGAMatchBuilderError, MatchResultBuilderError},
    replay::MatchReplayBuilderError,
};

/// A specialized Result type for `ArenaBuddy` operations.
///
/// This is a type alias for the standard library's [`Result`](core::result::Result) type with the
/// error type defaulting to [`Error`].
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum ParseError {
    #[error("No event found")]
    NoEvent,
    #[error("Parse error: {0}")]
    Error(String),
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Database file not found")]
    DatabaseNotFound,
    #[error("Could not decode data")]
    DecodeError,
    #[error("Could not encode data")]
    EncodeError,
    #[cfg(not(target_arch = "wasm32"))]
    #[error("Database migration failed {0}")]
    MigrationError(rusqlite_migration::Error),
    #[cfg(not(target_arch = "wasm32"))]
    #[error("SQLite error {0}")]
    SqliteError(rusqlite::Error),
    #[error("Json error {0}")]
    JsonError(serde_json::Error),
    #[error("Match Replay Build Error {0}")]
    MTGAMatchBuildError(MTGAMatchBuilderError),
    #[error("Match Replay Build Error {0}")]
    MatchReplayBuildError(MatchReplayBuilderError),
    #[error("Match Result Build Error {0}")]
    MatchResultBuildError(MatchResultBuilderError),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    ParseError(ParseError),
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::DatabaseNotFound
    }
}

impl From<prost::DecodeError> for Error {
    fn from(_: prost::DecodeError) -> Self {
        Error::DatabaseNotFound
    }
}

impl From<prost::EncodeError> for Error {
    fn from(_: prost::EncodeError) -> Self {
        Error::DatabaseNotFound
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<rusqlite_migration::Error> for Error {
    fn from(err: rusqlite_migration::Error) -> Self {
        Error::MigrationError(err)
    }
}

#[cfg(not(target_arch = "wasm32"))]
impl From<rusqlite::Error> for Error {
    fn from(err: rusqlite::Error) -> Self {
        Error::SqliteError(err)
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        Error::JsonError(err)
    }
}

impl From<MTGAMatchBuilderError> for Error {
    fn from(err: MTGAMatchBuilderError) -> Self {
        Error::MTGAMatchBuildError(err)
    }
}

impl From<MatchResultBuilderError> for Error {
    fn from(err: MatchResultBuilderError) -> Self {
        Error::MatchResultBuildError(err)
    }
}

impl From<MatchReplayBuilderError> for Error {
    fn from(err: MatchReplayBuilderError) -> Self {
        Error::MatchReplayBuildError(err)
    }
}

impl From<ParseError> for Error {
    fn from(err: ParseError) -> Self {
        Error::ParseError(err)
    }
}
