use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::{
    Error,
    errors::ParseError,
    processor::{EventSource, PlayerLogProcessor},
    replay::MatchReplayBuilder,
};
use arenabuddy_data::{DirectoryStorage, MatchDB, Storage};
use notify::{Event, Watcher};
use tokio::{
    sync::{Mutex, mpsc},
    time::sleep,
};
use tracing::{error, info};

async fn watch_player_log_rotation(
    notify_tx: mpsc::UnboundedSender<Event>,
    player_log_path: PathBuf,
) {
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            let _ = notify_tx.send(event);
        }
        Err(e) => {
            error!("watch error: {:?}", e);
        }
    })
    .expect("Could not create watcher");
    watcher
        .watch(&player_log_path, notify::RecursiveMode::NonRecursive)
        .expect("Could not watch player log path");
    loop {
        sleep(Duration::from_secs(1)).await;
    }
}

async fn log_process_start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    let (notify_tx, mut notify_rx) = mpsc::unbounded_channel::<Event>();
    let mut processor = PlayerLogProcessor::try_new(&player_log_path)
        .expect("Could not build player log processor");
    let mut match_replay_builder = MatchReplayBuilder::new();
    info!("Player log: {:?}", player_log_path);
    let plp = player_log_path.clone();

    tokio::spawn(async move {
        watch_player_log_rotation(notify_tx, plp).await;
    });

    loop {
        tokio::select! {
            event = notify_rx.recv() => {
                if let Some(event) = event {
                    info!("log file rotated!, {:?}", event);
                    processor = PlayerLogProcessor::try_new(&player_log_path)
                        .expect("Could not build player log processor");
                }
            }
            _ = sleep(Duration::from_secs(1)) => {
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
    }
}

pub fn start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    tokio::spawn(async move {
        log_process_start(db, debug_dir, log_collector, player_log_path).await;
    });
}
