use std::path::{Path, PathBuf};

use arenabuddy_core::{
    cards::CardsDatabase,
    player_log::{
        ingest::{IngestionConfig, IngestionEvent, LogIngestionService},
        replay::MatchReplay,
    },
};
use tracing::info;

use crate::Result;

pub async fn execute(
    player_log: &Path,
    cards_db_path: Option<&PathBuf>,
    output: Option<&PathBuf>,
    game_filter: Option<i32>,
) -> Result<()> {
    let cards_db = if let Some(path) = cards_db_path {
        CardsDatabase::new(path)?
    } else {
        CardsDatabase::default()
    };

    // Collect match replays by running the ingestion service with an event callback
    let replays = std::sync::Arc::new(tokio::sync::Mutex::new(Vec::<MatchReplay>::new()));
    let replays_clone = replays.clone();

    let callback: arenabuddy_core::player_log::ingest::EventCallback = std::sync::Arc::new(move |event| {
        let replays = replays_clone.clone();
        Box::pin(async move {
            if let IngestionEvent::MatchCompleted(replay) = event {
                replays.lock().await.push(*replay);
            }
        })
    });

    let config = IngestionConfig::new(player_log.to_path_buf())
        .with_follow(false)
        .with_rotation_watch(false);

    let service = LogIngestionService::new(config)
        .await?
        .with_event_callback(callback)
        .with_shutdown();

    info!("Processing Player.log: {:?}", player_log);
    service.start().await?;

    let replays = replays.lock().await;
    info!("Found {} match(es)", replays.len());

    let mut all_logs = Vec::new();

    for replay in replays.iter() {
        let (controller, opponent) = replay
            .get_player_names(replay.get_controller_seat_id())
            .unwrap_or(("Unknown".to_string(), "Unknown".to_string()));

        info!(
            "Match {}: {} vs {} ({} events)",
            replay.match_id,
            controller,
            opponent,
            replay.client_server_messages.len()
        );

        let mut event_logs = replay.get_event_logs(&cards_db);

        if let Some(game_num) = game_filter {
            event_logs.retain(|log| log.game_number == game_num);
        }

        for log in &event_logs {
            info!("  Game {}: {} events", log.game_number, log.events.len());
        }

        all_logs.extend(event_logs);
    }

    let json = serde_json::to_string_pretty(&all_logs).map_err(crate::ParseError::Json)?;

    if let Some(output_path) = output {
        std::fs::write(output_path, &json)?;
        info!("Event log written to {:?}", output_path);
    } else {
        println!("{json}");
    }

    Ok(())
}
