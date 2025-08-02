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
    #[error("proto decoding error")]
    ProtoEncodeError(#[from] prost::EncodeError),
    #[error("proto decoding error")]
    ProtoDecodeError(#[from] prost::DecodeError),
    #[error("Could not decode data")]
    DecodeError,
    #[error("Could not encode data")]
    EncodeError,
    #[error("Json error {0}")]
    JsonError(#[from] serde_json::Error),
    #[error("Match Replay Build Error {0}")]
    MTGAMatchBuildError(#[from] MTGAMatchBuilderError),
    #[error("Match Replay Build Error {0}")]
    MatchReplayBuildError(#[from] MatchReplayBuilderError),
    #[error("Match Result Build Error {0}")]
    MatchResultBuildError(#[from] MatchResultBuilderError),
    #[error("{0} not found")]
    NotFound(String),
    #[error("{0}")]
    Parse(#[from] ParseError),
    #[error("Storage error: {0}")]
    StorageError(String),
}

impl From<std::io::Error> for Error {
    fn from(_: std::io::Error) -> Self {
        Error::DatabaseNotFound
    }
}
