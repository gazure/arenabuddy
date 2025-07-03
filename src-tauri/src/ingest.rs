use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::Duration,
};

use arenabuddy_core::{
    Error,
    errors::ParseError,
    processor::{EventSource, PlayerLogProcessor},
    replay::MatchReplayBuilder,
    storage::{DirectoryStorageBackend, Storage},
};
use arenabuddy_data::MatchDB;
use crossbeam_channel::{Sender, select, unbounded};
use notify::{Event, Watcher};
use tracing::{error, info};

fn watch_player_log_rotation(notify_tx: Sender<Event>, player_log_path: &Path) {
    let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| match res {
        Ok(event) => {
            notify_tx.send(event).unwrap_or(());
        }
        Err(e) => {
            error!("watch error: {:?}", e);
        }
    })
    .expect("Could not create watcher");
    watcher
        .watch(player_log_path, notify::RecursiveMode::NonRecursive)
        .expect("Could not watch player log path");
    loop {
        std::thread::sleep(Duration::from_secs(1));
    }
}

fn log_process_start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorageBackend>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: &Path,
) {
    let (notify_tx, notify_rx) = unbounded::<Event>();
    let mut processor =
        PlayerLogProcessor::try_new(player_log_path).expect("Could not build player log processor");
    let mut match_replay_builder = MatchReplayBuilder::new();
    info!("Player log: {:?}", player_log_path);
    let plp = player_log_path.to_owned();

    std::thread::spawn(move || {
        watch_player_log_rotation(notify_tx, &plp);
    });

    loop {
        select! {
            recv(notify_rx) -> event => {
                if let Ok(event) = event {
                    info!("log file rotated!, {:?}", event);
                    processor = PlayerLogProcessor::try_new(player_log_path)
                        .expect("Could not build player log processor");
                }
            }
            default(Duration::from_secs(1)) => {
                loop {
                    match processor.get_next_event() {
                        Ok(parse_output) => {
                            if match_replay_builder.ingest(parse_output) {
                                let match_replay = match_replay_builder.build();
                                match match_replay {
                                    Ok(mr) => {
                                        let mut db = db.lock().expect("Could not lock db");
                                        if let Err(e) = db.write(&mr) {
                                            error!("Error writing match to db: {}", e);
                                        }

                                        let mut debug_dir = debug_dir.lock().expect("Could not lock debug dir");
                                        if let Some(dir) = debug_dir.as_mut() {
                                            if let Err(e) = dir.write(&mr) {
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
                                log_collector.lock().expect("log collector lock should be healthy").push(s);
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
    debug_dir: Arc<Mutex<Option<DirectoryStorageBackend>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    std::thread::spawn(move || {
        log_process_start(db, debug_dir, log_collector, &player_log_path);
    });
}
