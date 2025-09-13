use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use arenabuddy_core::{
    cards::CardsDatabase,
    player_log::ingest::{IngestionConfig, LogIngestionService},
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use tracingx::info;

use crate::Result;

/// Execute the Parse command
pub async fn execute(
    player_log: &Path,
    output_dir: Option<&PathBuf>,
    db: Option<&str>,
    cards_db_path: Option<&PathBuf>,
    follow: bool,
) -> Result<()> {
    // Load cards database
    let default_cards_db = PathBuf::from("data/cards-full.pb");
    let cards_db = CardsDatabase::new(cards_db_path.unwrap_or(&default_cards_db))?;

    // Configure the ingestion service
    let config = IngestionConfig::new(player_log.to_path_buf())
        .with_follow(follow)
        .with_rotation_watch(false); // CLI doesn't need rotation watching

    // Create the service
    let mut service = LogIngestionService::new(config).await?.with_shutdown();

    // Add directory storage if specified
    if let Some(output_dir) = output_dir {
        std::fs::create_dir_all(output_dir)?;
        info!("Writing replays to directory: {:?}", output_dir);
        let storage = DirectoryStorage::new(output_dir.clone());
        service = service.add_writer(Box::new(storage));
    }

    // Add database storage if specified
    if let Some(db_url) = db {
        info!("Writing replays to database: {}", db_url);
        let mut db = MatchDB::new(Some(db_url), Arc::new(cards_db)).await?;
        db.initialize().await?;
        service = service.add_writer(Box::new(db));
    }

    // Start processing
    info!("Starting log processing from: {:?}", player_log);
    if follow {
        info!("Following log file for new events (press Ctrl+C to stop)");
    } else {
        info!("Processing existing log entries");
    }

    service.start().await?;

    info!("Log processing completed");
    Ok(())
}
