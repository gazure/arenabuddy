use rustyline::error::ReadlineError;

#[derive(thiserror::Error, Debug)]
pub enum ParseError {
    #[error("JSON parse error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("CSV parse error: {0}")]
    Csv(#[from] csv::Error),
}

#[derive(thiserror::Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Parse error: {0}")]
    Parse(#[from] ParseError),

    #[error("{0}")]
    Invalid(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("{0}")]
    Db(#[from] arenabuddy_data::Error),

    #[error("Signal handler error: {0}")]
    Signal(#[from] ctrlc::Error),

    #[error("Core error: {0}")]
    Core(#[from] arenabuddy_core::Error),

    #[error("Network error: {0}")]
    Network(#[from] reqwest::Error),

    #[error("URL error: {0}")]
    Url(String),

    #[error("Readline error: {0}")]
    Readline(#[from] ReadlineError),

    #[error("Authentication failed")]
    Auth,

    #[error("Invalid command: {0}")]
    InvalidCommand(String),
}

pub type Result<T, E = Error> = core::result::Result<T, E>;
