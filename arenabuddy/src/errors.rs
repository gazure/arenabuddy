pub type Result<T, E = Error> = core::result::Result<T, E>;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Invalid input")]
    InvalidInput,
    #[error("Corrupted app data")]
    CorruptedAppData,
    #[error("Log failure")]
    LogFailure,
    #[error("Matches db failure")]
    MatchesDbFailure,
    #[error("No cards database")]
    NoCardsDatabase,
    #[error("No home dir")]
    NoHomeDir,
    #[error("No matches database")]
    NoMatchesDatabase,
    #[error("Unsupported operating system")]
    UnsupportedOS,
    #[error(transparent)]
    DbError(#[from] arenabuddy_data::Error),
    #[error(transparent)]
    CoreError(#[from] arenabuddy_core::Error),
    #[error("Io error: {0}")]
    IoError(String),
}

impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Error::IoError(err.to_string())
    }
}
