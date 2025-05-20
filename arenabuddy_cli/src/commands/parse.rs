use std::{path::PathBuf, time::Duration};

use anyhow::Result;
use arenabuddy_core::{
    cards::CardsDatabase,
    match_insights::MatchDB,
    processor::{EventSource, PlayerLogProcessor},
    replay::MatchReplayBuilder,
    storage_backends::{DirectoryStorageBackend, Storage},
};
use crossbeam::channel::{Receiver, select};
use tracing::{Level, error};
use tracing_subscriber::fmt;

// Constants
const PLAYER_LOG_POLLING_INTERVAL: u64 = 1;

/// Creates a channel that receives a signal when Ctrl+C is pressed
pub fn ctrl_c_channel() -> Result<Receiver<()>> {
    let (ctrl_c_tx, ctrl_c_rx) = crossbeam::channel::unbounded();
    ctrlc::set_handler(move || {
        ctrl_c_tx.send(()).unwrap_or(());
    })?;
    Ok(ctrl_c_rx)
}

/// Execute the Parse command
pub fn execute(
    player_log: &PathBuf,
    output_dir: &Option<PathBuf>,
    db: &Option<PathBuf>,
    cards_db: &Option<PathBuf>,
    debug: bool,
    follow: bool,
) -> Result<()> {
    // Initialize logging
    fmt()
        .with_max_level(if debug { Level::DEBUG } else { Level::INFO })
        .init();

    let mut processor = PlayerLogProcessor::try_new(player_log.clone())?;
    let mut match_replay_builder = MatchReplayBuilder::new();
    let mut storage_backends: Vec<Box<dyn Storage>> = Vec::new();
    let cards_db = CardsDatabase::new(
        cards_db
            .clone()
            .unwrap_or_else(|| PathBuf::from("data/cards-full.pb")),
    )?;

    let ctrl_c_rx = ctrl_c_channel()?;

    // Initialize directory storage backend if specified
    if let Some(output_dir) = output_dir {
        std::fs::create_dir_all(output_dir)?;
        storage_backends.push(Box::new(DirectoryStorageBackend::new(output_dir.clone())));
    }

    // Initialize database storage backend if specified
    if let Some(db_path) = db {
        let conn = rusqlite::Connection::open(db_path)?;
        let mut db = MatchDB::new(conn, cards_db);
        db.init()?;
        storage_backends.push(Box::new(db));
    }

    // Main processing loop
    loop {
        select! {
            recv(ctrl_c_rx) -> _ => {
                break;
            }
            default(Duration::from_secs(PLAYER_LOG_POLLING_INTERVAL)) => {
                while let Ok(parse_output) = processor.get_next_event() {
                    if match_replay_builder.ingest_event(parse_output) {
                        match match_replay_builder.build() {
                            Ok(match_replay) => {
                                for backend in &mut storage_backends {
                                    if let Err(e) = backend.write(&match_replay) {
                                        error!("Error writing replay to backend: {e}");
                                    }
                                }
                            },
                            Err(err) => {
                                error!("Error building match replay: {err}");
                            }
                        }
                        match_replay_builder = MatchReplayBuilder::new();
                    }
                }
                if !follow {
                    break;
                }
            }
        }
    }

    Ok(())
}
