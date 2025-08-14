use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::{
    Error as CoreError, errors::ParseError, processor::PlayerLogProcessor, replay::MatchReplayBuilder,
};
use arenabuddy_data::{DirectoryStorage, MatchDB, ReplayStorage};
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{
    Mutex,
    mpsc::{Receiver, channel},
};
use tracing::{error, info};

use crate::{Error, Result};

fn watcher() -> Result<(RecommendedWatcher, Receiver<Event>)> {
    let (tx, rx) = channel(100);

    info!("building watcher!");
    let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        info!("found event!");
        match res {
            Ok(event) => {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    info!("found event!");
                    let () = tx.send(event).await.expect("channel crashed");
                });
            }
            Err(e) => {
                error!("watch error: {:?}", e);
            }
        }
    })
    .map_err(|e| Error::IoError(e.to_string()))?;
    Ok((watcher, rx))
}

async fn log_process_start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) -> Result<()> {
    let mut processor = PlayerLogProcessor::try_new(&player_log_path).await?;
    let mut match_replay_builder = MatchReplayBuilder::new();
    let plp = player_log_path.clone();

    let (mut watcher, mut rx) = watcher()?;

    watcher
        .watch(plp.as_ref(), RecursiveMode::Recursive)
        .map_err(|e| Error::IoError(e.to_string()))?;
    info!("starting to ingest logs!");

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        tokio::select! {
            rotation = rx.recv() => {
                if let Some(event) = rotation {
                    info!("log file rotated!, {:?}", event);
                    processor = PlayerLogProcessor::try_new(&player_log_path).await?;
                } else {
                    error!("disconnected rotation channel");
                    return Err(Error::IoError("disconnected rotation channel".to_string()));
                }
            }
            _ = interval.tick() => {
                loop {
                    match processor.get_next_event().await {
                        Ok(parse_output) => {
                            if match_replay_builder.ingest(parse_output) {
                                let match_replay = match_replay_builder.build();
                                match match_replay {
                                    Ok(mr) => {
                                        let mut db = db.lock().await;
                                        if let Err(e) = db.write(&mr).await {
                                            error!("Error writing match to db: {}", e);
                                        }

                                        let mut debug_dir = debug_dir.lock().await;
                                        if let Some(dir) = debug_dir.as_mut()
                                            && let Err(e) = dir.write(&mr).await {
                                                error!("Error writing match to debug dir: {}", e);
                                            }
                                    }
                                    Err(e) => {
                                        error!("Error building match replay: {}", e);
                                    }
                                }
                                match_replay_builder = MatchReplayBuilder::new();
                            }
                        }
                        Err(parse_error) => {
                            if let CoreError::Parse(ParseError::Error(s)) = parse_error {
                                log_collector.lock().await.push(s);
                            } else {
                                break;
                            }
                        }
                    }
                }
            }
        }
    }
}

pub async fn start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    let res = log_process_start(db, debug_dir, log_collector, player_log_path).await;
    if let Err(e) = res {
        error!("Log processing failed: {}", e);
    }
}
