use std::path::{Path, PathBuf};

use arenabuddy_core::{
    cards::CardsDatabase,
    player_log::ingest::{IngestionConfig, LogIngestionService},
};
use arenabuddy_data::{Database, DirectoryStorage};
use tracing::info;

use crate::Result;

/// Execute the Parse command
pub async fn execute(
    player_log: &Path,
    output_dir: Option<&PathBuf>,
    db: Option<&str>,
    cards_db_path: Option<&PathBuf>,
    follow: bool,
) -> Result<()> {
    let default_cards_db = PathBuf::from("data/cards-full.pb");
    let cards_db = CardsDatabase::new(cards_db_path.unwrap_or(&default_cards_db))?;

    let config = IngestionConfig::new(player_log.to_path_buf())
        .with_follow(follow)
        .with_rotation_watch(false); // Follow rotations by polling in the CLI

    let mut service = LogIngestionService::new(config).await?.with_shutdown();

    if let Some(output_dir) = output_dir {
        std::fs::create_dir_all(output_dir)?;
        info!("Writing replays to directory: {:?}", output_dir);
        let storage = DirectoryStorage::new(output_dir.clone());
        service = service.add_writer(Box::new(storage));
    }

    if let Some(db_url) = db {
        info!("Writing replays to database: {}", db_url);
        let database = Database::connect(db_url).await?;
        database.migrate().await?;
        let db = database.repository(cards_db);
        service = service.add_writer(Box::new(db));
    }

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
