#![expect(clippy::needless_pass_by_value)]
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use std::{
    error::Error,
    fmt::Display,
    path::{Path, PathBuf},
    sync::Arc,
};

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{DirectoryStorage, MatchDB};
use dioxus::{
    desktop::{Config, WindowBuilder},
    prelude::*,
};
use serde::{Deserialize, Serialize};
use start::AppMeta;
use tracing::{Level, debug, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

mod app;
mod components;
mod debug_logs;
mod error_logs;
#[cfg(feature = "server")]
mod ingest;
mod match_details;
mod matches;
mod service;
mod state;

use app::App;
#[cfg(feature = "server")]
use service::AppService;

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

fn get_app_data_dir() -> Result<std::path::PathBuf, Box<dyn Error>> {
    let home = std::env::home_dir().ok_or(ArenaBuddyError::NoHomeDir)?;

    let app_data = match std::env::consts::OS {
        "macos" => home.join("Library/Application Support/com.gazure.dev.arenabuddy.app"),
        "windows" => home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app"),
        "linux" => home.join(".local/share/com.gazure.dev.arenabuddy.app"),
        _ => return Err(Box::new(ArenaBuddyError::UnsupportedOS)),
    };

    std::fs::create_dir_all(&app_data).map_err(|_| ArenaBuddyError::CorruptedAppData)?;
    Ok(app_data)
}

fn get_resource_dir() -> Result<std::path::PathBuf, Box<dyn Error>> {
    // In a desktop app, resources are typically bundled with the executable
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();

    // Check common resource locations
    let possible_paths = vec![
        exe_dir.join("resources"),
        exe_dir.join("data"),
        exe_dir.parent().unwrap().join("Resources"), // macOS app bundle
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(
            |_| std::env::current_dir().unwrap().join("./data"),
            |dir| std::path::PathBuf::from(dir).join("data"),
        ), // Development mode
    ];

    for path in possible_paths {
        debug!("looking in: {:?}", path);
        if path.join("cards-full.pb").exists() {
            return Ok(path);
        }
    }

    Err(Box::new(ArenaBuddyError::NoCardsDatabase))
}

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
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    let console_layer = fmt::Layer::new()
        .with_target(true)
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    let console_filter = EnvFilter::new("info");

    registry
        .with(file_layer)
        .with(console_layer.with_filter(console_filter))
        .init();
    Ok(())
}

#[cfg(feature = "server")]
async fn build_service(app_data_dir: PathBuf, resource_dir: PathBuf) -> Result<AppService, Box<dyn Error>> {
    setup_logging(&app_data_dir)?;
    let app_meta = AppMeta::from_env().with_app_name("arenabuddy");
    let root_span = tracing::info_span!("app", app = %app_meta.app);
    let _span = root_span.enter();

    info!("resource dir: {:?}", resource_dir);
    let cards_path = resource_dir.join("cards-full.pb");
    info!("cards_db path: {:?}", cards_path);
    let cards_db = CardsDatabase::new(cards_path).map_err(|_| ArenaBuddyError::NoCardsDatabase)?;

    let url = std::env::var("ARENABUDDY_DATABASE_URL").ok();
    info!("using matches db: {:?}", url);
    let mut db = MatchDB::new(url.as_deref(), cards_db).await?;
    db.init().await?;
    let db_arc = Arc::new(tokio::sync::Mutex::new(db));

    let log_collector = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let debug_backend = Arc::new(tokio::sync::Mutex::new(None::<DirectoryStorage>));

    // Initialize global service
    let service = AppService::new(db_arc.clone(), log_collector.clone(), debug_backend.clone());

    Ok(service)
}

#[cfg(feature = "server")]
async fn create_app_service() -> Result<AppService, Box<dyn std::error::Error>> {
    let data_dir = get_app_data_dir()?;
    let resource_dir = get_resource_dir()?;
    build_service(data_dir, resource_dir).await
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "server")]
    {
        let data_dir = get_app_data_dir()?;
        let resource_dir = get_resource_dir()?;
        let home = std::env::home_dir().ok_or(ArenaBuddyError::NoHomeDir)?;
        let player_log_path = match std::env::consts::OS {
            "macos" => Ok(home.join("Library/Logs/Wizards of the Coast/MTGA/Player.log")),
            "windows" => Ok(home.join("AppData/LocalLow/Wizards of the Coast/MTGA/Player.log")),
            _ => Err(ArenaBuddyError::UnsupportedOS),
        }?;
        info!("Processing logs from : {}", player_log_path.to_string_lossy());

        let background = tokio::runtime::Runtime::new()?;
        let service = background.block_on(create_app_service())?;
        let service2 = service.clone();
        background.spawn(async move {
            ingest::start(
                service2.db.clone(),
                service2.debug_storage.clone(),
                service2.log_collector.clone(),
                player_log_path,
            )
            .await;
        });

        LaunchBuilder::server()
            .with_cfg(
                Config::new()
                    .with_data_directory(data_dir.clone())
                    .with_resource_directory(resource_dir.clone())
                    .with_window(WindowBuilder::new().with_resizable(true)),
            )
            .with_context(service)
            .launch(App);
    }

    #[cfg(not(feature = "server"))]
    {
        LaunchBuilder::desktop().launch(App);
    }

    Ok(())
}
