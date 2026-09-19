use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::{
    player_log::{
        ingest::{IngestionConfig, IngestionEvent, LogIngestionService},
        replay::MatchReplay,
    },
    services::debug_service::{ParseErrorReport, ReportParseErrorsRequest, debug_service_client::DebugServiceClient},
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use tokio::sync::{Mutex, mpsc};
use tonic::transport::Endpoint;
use tracing::{error, info};

use super::auth::SharedAuthState;

/// Adapter that wraps shared debug storage for the `ReplayWriter` trait.
///
/// The `Arc<Mutex<Option<..>>>` wrapping is intentional: the storage may not
/// be configured at startup (the user sets the directory later via the UI),
/// and both `AppService` and the ingestion service need shared mutable access.
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

struct QueuedReplayWriter {
    db: MatchDB,
    auth: SharedAuthState,
}

#[async_trait::async_trait]
impl arenabuddy_core::player_log::ingest::ReplayWriter for QueuedReplayWriter {
    async fn write(&mut self, replay: &MatchReplay) -> arenabuddy_core::Result<()> {
        let account = self.auth.lock().await.as_ref().map(|state| state.user.id.clone());
        self.db
            .write_replay_for_upload(replay, account.as_deref())
            .await
            .map_err(|e| arenabuddy_core::Error::StorageError(e.to_string()))
    }
}

// Diagnostics are best effort and bounded; network failures never delay parsing.
async fn report_errors(mut errors: mpsc::Receiver<(String, String)>) {
    while let Some((raw_json, token)) = errors.recv().await {
        let report = async {
            let endpoint = Endpoint::new(super::paths::grpc_url())?.connect_timeout(Duration::from_secs(5));
            let mut client = DebugServiceClient::new(endpoint.connect().await?);
            let mut request = tonic::Request::new(ReportParseErrorsRequest {
                errors: vec![ParseErrorReport {
                    raw_json,
                    timestamp: chrono::Utc::now().timestamp(),
                }],
            });
            super::auth::attach_bearer(&mut request, Some(&token));
            request.set_timeout(Duration::from_secs(10));
            client.report_parse_errors(request).await?;
            Ok::<(), Box<dyn std::error::Error + Send + Sync>>(())
        };
        match tokio::time::timeout(Duration::from_secs(15), report).await {
            Ok(Ok(())) => {}
            Ok(Err(e)) => error!("Parse error reporting failed: {e}"),
            Err(e) => error!("Parse error reporting timed out: {e}"),
        }
    }
}

pub async fn start(
    db: MatchDB,
    debug_dir: Arc<Mutex<Option<DirectoryStorage>>>,
    log_collector: Arc<Mutex<Vec<String>>>,
    player_log_path: PathBuf,
    auth_state: SharedAuthState,
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

    let service = service
        .add_writer(Box::new(QueuedReplayWriter {
            db: db.clone(),
            auth: auth_state.clone(),
        }))
        .add_draft_writer(Box::new(db))
        .add_writer(Box::new(DirectoryStorageAdapter::new(debug_dir)));

    let (error_tx, error_rx) = mpsc::channel(64);
    let reporter = tokio::spawn(report_errors(error_rx));
    let event_callback: arenabuddy_core::player_log::ingest::EventCallback = Arc::new(move |event| {
        let logs = log_collector.clone();
        let sender = error_tx.clone();
        let auth = auth_state.clone();
        Box::pin(async move {
            if let IngestionEvent::ParseError(raw_json) = event {
                {
                    let mut logs = logs.lock().await;
                    if logs.len() >= 1000 {
                        logs.remove(0);
                    }
                    logs.push(raw_json.clone());
                }
                let token = auth.lock().await.as_ref().map(|state| state.token.clone());
                if let Some(token) = token {
                    let _ = sender.try_send((raw_json, token));
                }
            }
        })
    });

    let service = service.with_event_callback(event_callback);

    // Start the service
    if let Err(e) = service.start().await {
        error!("Log processing failed: {}", e);
    }
    reporter.abort();
}
