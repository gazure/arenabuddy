use std::{
    path::{Path, PathBuf},
    time::Duration,
};

use anyhow::Result;
use arenabuddy_core::{
    cards::CardsDatabase,
    processor::{EventSource, PlayerLogProcessor},
    replay::MatchReplayBuilder,
};
use arenabuddy_data::{DirectoryStorage, MatchDB, Storage};
use tokio::{sync::mpsc, time::sleep};
use tracing::{Level, error};

// Constants
const PLAYER_LOG_POLLING_INTERVAL: Duration = Duration::from_secs(1);

/// Process events from the player log and handle match replays
async fn process_events(
    processor: &mut PlayerLogProcessor,
    mut match_replay_builder: MatchReplayBuilder,
    directory_storage: &mut Option<DirectoryStorage>,
    db: &mut Option<MatchDB>,
    follow: bool,
) -> Result<Option<MatchReplayBuilder>> {
    while let Ok(event) = processor.get_next_event() {
        if match_replay_builder.ingest(event) {
            match match_replay_builder.build() {
                Ok(match_replay) => {
                    if let Some(dir) = directory_storage {
                        dir.write(&match_replay).await?;
                    }
                    if let Some(db) = db {
                        db.write(&match_replay).await?
                    }
                }
                Err(err) => {
                    error!("Error building match replay: {err}");
                }
            }
            match_replay_builder = MatchReplayBuilder::new();
        }
    }
    if follow {
        Ok(Some(match_replay_builder))
    } else {
        Ok(None)
    }
}

/// Creates a channel that receives a signal when Ctrl+C is pressed</parameter>
pub fn ctrl_c_channel() -> Result<mpsc::UnboundedReceiver<()>> {
    let (ctrl_c_tx, ctrl_c_rx) = mpsc::unbounded_channel();
    ctrlc::set_handler(move || {
        let _ = ctrl_c_tx.send(());
    })?;
    Ok(ctrl_c_rx)
}

/// Execute the Parse command
pub async fn execute(
    player_log: &Path,
    output_dir: &Option<PathBuf>,
    db: &Option<String>,
    cards_db: &Option<PathBuf>,
    debug: bool,
    follow: bool,
) -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(if debug { Level::DEBUG } else { Level::INFO })
        .init();

    let mut processor = PlayerLogProcessor::try_new(player_log)?;
    let mut match_replay_builder = MatchReplayBuilder::new();
    let cards_db = CardsDatabase::new(
        cards_db
            .clone()
            .unwrap_or_else(|| PathBuf::from("data/cards-full.pb")),
    )?;

    let mut ctrl_c_rx = ctrl_c_channel()?;

    // Initialize directory storage backend if specified
    let mut directory_storage = if let Some(output_dir) = output_dir {
        std::fs::create_dir_all(output_dir)?;
        Some(DirectoryStorage::new(output_dir.clone()))
    } else {
        None
    };

    // Initialize database storage backend if specified
    let mut db = if let Some(db_url) = db {
        let mut db = MatchDB::new(Some(db_url), cards_db).await?;
        db.init().await?;
        Some(db)
    } else {
        None
    };

    // Main processing loop
    loop {
        tokio::select! {
            _ = ctrl_c_rx.recv() => {
                break;
            }
            _ = sleep(PLAYER_LOG_POLLING_INTERVAL) => {
                match process_events(
                    &mut processor,
                    match_replay_builder,
                    &mut directory_storage,
                    &mut db,
                    follow,
                ).await? {
                    Some(builder) => match_replay_builder = builder,
                    None => break,
                }
            }
        }
    }

    Ok(())
}
