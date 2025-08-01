// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_pass_by_value)]

use std::{error::Error, fmt::Display, path::Path, sync::Arc};

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{DirectoryStorage, MatchDB};
use serde::{Deserialize, Serialize};
use tauri::{App, Manager, path::BaseDirectory};
use tokio::sync::Mutex;
use tracing::{Level, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod commands;
mod ingest;

#[derive(Debug, Deserialize, Serialize)]
pub enum ArenaBuddyError {
    CorruptedAppData,
    LogSetupFailure,
    MatchesDatabaseInitializationFailure,
    NoCardsDatabase,
    NoHomeDir,
    NoMathchesDatabase,
    UnsupportedOS,
}

impl Display for ArenaBuddyError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::CorruptedAppData => write!(f, "App data is corrupted"),
            Self::LogSetupFailure => write!(f, "Could not setup logging"),
            Self::MatchesDatabaseInitializationFailure => {
                write!(f, "Matches db initialization failure")
            }
            Self::NoCardsDatabase => write!(f, "Cards database not found"),
            Self::NoHomeDir => write!(f, "Home directory not found"),
            Self::NoMathchesDatabase => write!(f, "Matches database not found"),
            Self::UnsupportedOS => write!(f, "Unsupported operating system"),
        }
    }
}

impl Error for ArenaBuddyError {}

fn setup_logging(app_data_dir: &Path) -> Result<(), Box<dyn Error>> {
    let registry = tracing_subscriber::registry();
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|_| ArenaBuddyError::CorruptedAppData)?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("arena-buddy")
        .build(log_dir)
        .map_err(|_| ArenaBuddyError::LogSetupFailure)?
        .with_max_level(Level::INFO);

    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(false)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    let console_layer = fmt::Layer::new()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    registry.with(file_layer).with(console_layer).init();
    Ok(())
}

async fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| ArenaBuddyError::CorruptedAppData)?;
    std::fs::create_dir_all(&app_data_dir).map_err(|_| ArenaBuddyError::CorruptedAppData)?;

    setup_logging(&app_data_dir)?;

    let cards_path = app
        .path()
        .resolve("./data/cards-full.pb", BaseDirectory::Resource)
        .map_err(|_| ArenaBuddyError::NoCardsDatabase)?;
    info!("cards_db path: {:?}", cards_path);
    let cards_db = CardsDatabase::new(cards_path).map_err(|_| ArenaBuddyError::NoCardsDatabase)?;

    let db_path = app_data_dir.join("matches.db");
    info!("Database path: {}", db_path.to_string_lossy());
    let url = std::env::var("DATABASE_URL").ok();

    let mut db = MatchDB::new(url.as_deref(), cards_db).await?;
    db.init().await?;
    let db_arc = Arc::new(Mutex::new(db));

    let home = app
        .path()
        .home_dir()
        .map_err(|_| ArenaBuddyError::NoHomeDir)?;
    let player_log_path = match std::env::consts::OS {
        "macos" => Ok(home.join("Library/Logs/Wizards of the Coast/MTGA/Player.log")),
        "windows" => Ok(home.join("AppData/LocalLow/Wizards of the Coast/MTGA/Player.log")),
        _ => Err(ArenaBuddyError::UnsupportedOS),
    }?;

    app.manage(db_arc.clone());
    info!(
        "Processing logs from : {}",
        player_log_path.to_string_lossy()
    );

    let log_collector = Arc::new(Mutex::new(Vec::<String>::new()));
    app.manage(log_collector.clone());

    let debug_backend = Arc::new(Mutex::new(None::<DirectoryStorage>));
    app.manage(debug_backend.clone());

    ingest::start(
        db_arc.clone(),
        debug_backend.clone(),
        log_collector,
        player_log_path,
    );
    Ok(())
}

/// # Errors
/// Will return an error if the tauri runtime fails
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| tauri::async_runtime::block_on(setup(app)))
        .invoke_handler(tauri::generate_handler![
            commands::matches::command_matches,
            commands::match_details::command_match_details,
            commands::error_logs::command_error_logs,
            commands::debug_logs::command_set_debug_logs,
            commands::debug_logs::command_get_debug_logs,
        ])
        .run(tauri::generate_context!())
}
