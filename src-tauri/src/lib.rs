// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]
#![deny(clippy::pedantic)]
#![deny(clippy::unwrap_used)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::too_many_arguments)]
#![allow(clippy::needless_pass_by_value)]

use std::error::Error;
use std::fmt::Display;
use std::sync::{Arc, Mutex};

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_core::match_insights::MatchDB;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use tauri::{App, Manager, path::BaseDirectory};
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
pub enum ArenaBuddySetupError {
    CorruptedAppData,
    LogSetupFailure,
    MatchesDatabaseInitializationFailure,
    NoCardsDatabase,
    NoHomeDir,
    NoMathchesDatabase,
    UnsupportedOS,
}

impl Display for ArenaBuddySetupError {
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

impl Error for ArenaBuddySetupError {}

fn setup(app: &mut App) -> Result<(), Box<dyn Error>> {
    let registry = tracing_subscriber::registry();
    let app_data_dir = app
        .path()
        .app_data_dir()
        .map_err(|_| ArenaBuddySetupError::CorruptedAppData)?;
    std::fs::create_dir_all(&app_data_dir).map_err(|_| ArenaBuddySetupError::CorruptedAppData)?;

    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|_| ArenaBuddySetupError::CorruptedAppData)?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("arena-buddy")
        .build(log_dir)
        .map_err(|_| ArenaBuddySetupError::LogSetupFailure)?
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

    let cards_path = app
        .path()
        .resolve("./data/cards-full.json", BaseDirectory::Resource)
        .map_err(|_| ArenaBuddySetupError::NoCardsDatabase)?;
    info!("cards_db path: {:?}", cards_path);
    let cards_db =
        CardsDatabase::new(cards_path).map_err(|_| ArenaBuddySetupError::NoCardsDatabase)?;

    let ruby = cards_db.get("93958");
    info!("Ruby: {:?}", ruby);

    let db_path = app_data_dir.join("matches.db");
    info!("Database path: {}", db_path.to_string_lossy());
    let conn = Connection::open(db_path).map_err(|_| ArenaBuddySetupError::NoMathchesDatabase)?;
    let mut db = MatchDB::new(conn, cards_db);
    db.init()
        .map_err(|_| ArenaBuddySetupError::MatchesDatabaseInitializationFailure)?;
    let db_arc = Arc::new(Mutex::new(db));

    let home = app
        .path()
        .home_dir()
        .map_err(|_| ArenaBuddySetupError::NoHomeDir)?;
    let os = std::env::consts::OS;
    let player_log_path = match os {
        "macos" => Ok(home.join("Library/Logs/Wizards of the Coast/MTGA/Player.log")),
        "windows" => Ok(home.join("AppData/LocalLow/Wizards of the Coast/MTGA/Player.log")),
        _ => Err(ArenaBuddySetupError::UnsupportedOS),
    }?;

    app.manage(db_arc.clone());
    info!(
        "Processing logs from : {}",
        player_log_path.to_string_lossy()
    );

    let log_collector = Arc::new(Mutex::new(Vec::<String>::new()));

    app.manage(log_collector.clone());

    ingest::start(db_arc.clone(), log_collector, player_log_path);
    Ok(())
}

/// # Errors
/// Will return an error if the tauri runtime fails
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() -> tauri::Result<()> {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(setup)
        .invoke_handler(tauri::generate_handler![
            commands::matches::command_matches,
            commands::match_details::command_match_details,
            commands::error_logs::command_error_logs,
        ])
        .run(tauri::generate_context!())
}
