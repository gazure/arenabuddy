use std::{path::PathBuf, sync::Arc, time::Duration};

use anyhow::{anyhow, Result};
use arenabuddy_core::{
    errors::ParseError,
    processor::{EventSource, PlayerLogProcessor},
    replay::MatchReplayBuilder,
    Error,
};
use arenabuddy_data::{DirectoryStorage, MatchDB, Storage};
use notify::{Event, FsEventWatcher, RecursiveMode, Watcher};
use tokio::sync::{
    mpsc::{channel, error::TryRecvError, Receiver},
    Mutex,
};
use tracing::{error, info};

fn watcher() -> Result<(FsEventWatcher, Receiver<Event>)> {
    let (tx, rx) = channel(100);

    info!("building watcher!");
    let watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
        info!("found event!");
        match res {
            Ok(event) => {
                tokio::runtime::Runtime::new().unwrap().block_on(async {
                    info!("found event!");
                    let _ = tx.send(event).await.expect("channel crashed");
                });
            }
            Err(e) => {
                error!("watch error: {:?}", e);
            }
        }
    })?;
    Ok((watcher, rx))
}

async fn log_process_start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) -> Result<()> {
    let mut processor = PlayerLogProcessor::try_new(&player_log_path)
        .expect("Could not build player log processor");
    let mut match_replay_builder = MatchReplayBuilder::new();
    info!("Player log: {:?}", player_log_path);
    let plp = player_log_path.clone();

    let (mut watcher, mut rx) = watcher()?;

    watcher.watch(plp.as_ref(), RecursiveMode::Recursive)?;

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    loop {
        let rotation = rx.try_recv();
        match rotation {
            Ok(event) => {
                info!("log file rotated!, {:?}", event);
                processor = PlayerLogProcessor::try_new(&player_log_path)
                    .expect("Could not build player log processor");
            }
            Err(TryRecvError::Empty) => {
                info!("no log rotation");
            }
            Err(TryRecvError::Disconnected) => {
                error!("discconnected rotation channel");
                return Err(anyhow!("discconnected rotation channel"));
            }
        }
        interval.tick().await;
        loop {
            match processor.get_next_event() {
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
                                if let Some(dir) = debug_dir.as_mut() {
                                    if let Err(e) = dir.write(&mr).await {
                                        error!("Error writing match to debug dir: {}", e);
                                    }
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
                    if let Error::Parse(ParseError::Error(s)) = parse_error {
                        log_collector.lock().await.push(s);
                    } else {
                        break;
                    }
                }
            }
        }
    }
}

pub fn start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    tokio::spawn(async move {
        let res = log_process_start(db, debug_dir, log_collector, player_log_path).await;
        if let Err(e) = res {
            error!("Log processing failed: {}", e);
        }
    });
}
