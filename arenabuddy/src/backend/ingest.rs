use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::{
    models::MTGADraft,
    player_log::{
        ingest::{IngestionConfig, IngestionEvent, LogIngestionService},
        replay::MatchReplay,
    },
};
use arenabuddy_data::{ArenabuddyRepository, DirectoryStorage, MatchDB};
use tokio::sync::Mutex;
use tracingx::{error, info};

/// Adapter that wraps an Arc<Mutex<MatchDB>> for the `ReplayWriter` trait
#[derive(Clone)]
struct ArcMatchDBAdapter {
    db: Arc<Mutex<MatchDB>>,
}

impl ArcMatchDBAdapter {
    fn new(db: Arc<Mutex<MatchDB>>) -> Self {
        Self { db }
    }
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::ReplayWriter for ArcMatchDBAdapter {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        let mut db = self.db.lock().await;
        db.write_replay(replay)
            .await
            .map_err(|e| arenabuddy_core::Error::StorageError(e.to_string()))
    }
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::DraftWriter for ArcMatchDBAdapter {
    async fn write(&mut self, draft: &MTGADraft) -> arenabuddy_core::Result<()> {
        let mut db = self.db.lock().await;
        db.write_draft(draft)
            .await
            .map_err(|e| arenabuddy_core::Error::StorageError(e.to_string()))
    }
}

/// Adapter that wraps an Arc<Mutex<Option<DirectoryStorage>>> for the `ReplayWriter` trait
/// TODO: rethink this
struct DirectoryStorageAdapter {
    storage: Arc<Mutex<Option<DirectoryStorage>>>,
}

impl DirectoryStorageAdapter {
    fn new(storage: Arc<Mutex<Option<DirectoryStorage>>>) -> Self {
        Self { storage }
    }
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::ReplayWriter for DirectoryStorageAdapter {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        let mut storage = self.storage.lock().await;
        if let Some(dir) = storage.as_mut() {
            dir.write(replay)
                .await
                .map_err(|e| arenabuddy_core::Error::StorageError(e.to_string()))
        } else {
            Ok(())
        }
    }
}

pub async fn start(
    db: Arc<Mutex<MatchDB>>,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
) {
    info!("Initializing log ingestion service");

    // Configure the ingestion service
    let config = IngestionConfig::new(player_log_path.clone())
        .with_follow(true)
        .with_poll_interval(Duration::from_secs(1))
        .with_rotation_watch(true);

    // Create the service
    let service = match LogIngestionService::new(config).await {
        Ok(service) => service,
        Err(e) => {
            error!("Failed to create log ingestion service: {}", e);
            return;
        }
    };

    // Add database writer
    let db_adapter = ArcMatchDBAdapter::new(db.clone());
    let service = service
        .add_writer(Box::new(db_adapter.clone()))
        .add_draft_writer(Box::new(db_adapter));

    // Add directory storage writer
    let dir_adapter = DirectoryStorageAdapter::new(debug_dir.clone());
    let service = service.add_writer(Box::new(dir_adapter));

    // Set up event callback to handle draft events and collect errors
    let log_collector_clone = log_collector.clone();
    let event_callback = Arc::new(move |event: IngestionEvent| {
        if let IngestionEvent::ParseError(error) = event {
            // Collect parse errors like the original implementation
            let mut collector = log_collector_clone.blocking_lock();
            collector.push(error);
        }
    });

    let service = service.with_event_callback(event_callback);

    // Start the service
    if let Err(e) = service.start().await {
        error!("Log processing failed: {}", e);
    }
}
