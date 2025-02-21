use std::path::PathBuf;
use std::time::Duration;

use anyhow::Result;
use clap::Parser;
use crossbeam::channel::{Receiver, select, unbounded};
use tracing::error;

use arenabuddy_core::match_insights::MatchDB;
use arenabuddy_core::processor::{EventSource, PlayerLogProcessor};
use arenabuddy_core::replay::MatchReplayBuilder;
use arenabuddy_core::storage_backends::{DirectoryStorageBackend, Storage};

const PLAYER_LOG_POLLING_INTERVAL: u64 = 1;

#[derive(Debug, Parser)]
#[command(about = "Tries to scrape useful data from mtga detailed logs")]
struct Args {
    #[arg(short, long, help = "Location of Player.log file")]
    player_log: PathBuf,
    #[arg(short, long, help = "directory to write replay output files")]
    output_dir: Option<PathBuf>,
    #[arg(short, long, help = "database to write match data to")]
    db: Option<PathBuf>,
    #[arg(short, long, help = "database of cards to reference")]
    cards_db: Option<PathBuf>,
    #[arg(long, action = clap::ArgAction::SetTrue, help = "enable debug logging")]
    debug: bool,
    #[arg(
        short, long, action = clap::ArgAction::SetTrue, help = "wait for new events on Player.log, useful if you are actively playing MTGA"
    )]
    follow: bool,
}

fn ctrl_c_channel() -> Result<Receiver<()>> {
    let (ctrl_c_tx, ctrl_c_rx) = unbounded();
    ctrlc::set_handler(move || {
        ctrl_c_tx.send(()).unwrap_or(());
    })?;
    Ok(ctrl_c_rx)
}

fn main() -> Result<()> {
    let args = Args::try_parse()?;
    tracing_subscriber::fmt()
        .with_max_level(if args.debug {
            tracing::Level::DEBUG
        } else {
            tracing::Level::INFO
        })
        .init();

    let mut processor = PlayerLogProcessor::try_new(args.player_log)?;
    let mut match_replay_builder = MatchReplayBuilder::new();
    let mut storage_backends: Vec<Box<dyn Storage>> = Vec::new();
    let cards_db = arenabuddy_core::cards::CardsDatabase::new(
        args.cards_db.unwrap_or("data/merged.json".into()),
    )?;

    let ctrl_c_rx = ctrl_c_channel()?;
    if let Some(output_dir) = args.output_dir {
        std::fs::create_dir_all(&output_dir)?;
        storage_backends.push(Box::new(DirectoryStorageBackend::new(output_dir)));
    }

    if let Some(db_path) = args.db {
        let conn = rusqlite::Connection::open(db_path)?;
        let mut db = MatchDB::new(conn, cards_db);
        db.init()?;
        storage_backends.push(Box::new(db));
    }

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
                if !args.follow {
                    break;
                }
            }
        }
    }

    Ok(())
}
