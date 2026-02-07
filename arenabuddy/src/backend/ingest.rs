use std::{path::PathBuf, sync::Arc, time::Duration};

use arenabuddy_core::{
    cards::CardsDatabase,
    player_log::{
        ingest::{IngestionConfig, IngestionEvent, LogIngestionService},
        replay::MatchReplay,
    },
    services::debug_service::{ParseErrorReport, ReportParseErrorsRequest, debug_service_client::DebugServiceClient},
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use tokio::sync::Mutex;
use tonic::transport::Channel;
use tracingx::{error, info};

use super::{auth::SharedAuthState, grpc_writer::GrpcReplayWriter};

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

/// Type alias for cleaner async callback syntax
type PinnedFuture = std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>>;

/// Helper function to handle ingestion events
fn handle_ingestion_event(
    event: IngestionEvent,
    log_collector: Arc<Mutex<Vec<String>>>,
    debug_client: Option<Arc<Mutex<DebugReporter>>>,
) -> PinnedFuture {
    Box::pin(async move {
        tracingx::debug!("{event}");
        if let IngestionEvent::ParseError(raw_json) = event {
            {
                let mut collector = log_collector.lock().await;
                collector.push(raw_json.clone());
            }
            if let Some(reporter) = debug_client {
                let mut reporter = reporter.lock().await;
                reporter.report_parse_error(&raw_json).await;
            }
        }
    })
}

struct DebugReporter {
    client: DebugServiceClient<Channel>,
    token: Option<String>,
}

impl DebugReporter {
    async fn report_parse_error(&mut self, raw_json: &str) {
        let timestamp = chrono::Utc::now().timestamp();
        let mut request = tonic::Request::new(ReportParseErrorsRequest {
            errors: vec![ParseErrorReport {
                raw_json: raw_json.to_string(),
                timestamp,
            }],
        });

        if let Some(token) = &self.token {
            let bearer = format!("Bearer {token}");
            if let Ok(value) = bearer.parse() {
                request.metadata_mut().insert("authorization", value);
            }
        }

        if let Err(e) = self.client.report_parse_errors(request).await {
            error!("Failed to report parse error to server: {e}");
        }
    }
}

pub async fn start(
    db: MatchDB,
    cards: Arc<CardsDatabase>,
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

    // Add database writer
    let service = service.add_writer(Box::new(db.clone())).add_draft_writer(Box::new(db));

    // Add directory storage writer
    let dir_adapter = DirectoryStorageAdapter::new(debug_dir.clone());
    let service = service.add_writer(Box::new(dir_adapter));

    // Add gRPC writer and debug reporter if URL is configured
    let mut debug_reporter: Option<Arc<Mutex<DebugReporter>>> = None;
    let service = if let Ok(grpc_url) = std::env::var("ARENABUDDY_GRPC_URL") {
        let token = auth_state.lock().await.as_ref().map(|s| s.token.clone());
        match GrpcReplayWriter::connect(&grpc_url, cards, token.clone()).await {
            Ok(writer) => {
                info!("Connected to gRPC backend at {grpc_url}");

                // Create a separate debug client
                if let Ok(client) = DebugServiceClient::connect(grpc_url).await {
                    debug_reporter = Some(Arc::new(Mutex::new(DebugReporter { client, token })));
                }

                service.add_writer(Box::new(writer))
            }
            Err(e) => {
                error!("Failed to connect to gRPC backend at {grpc_url}: {e}");
                service
            }
        }
    } else {
        service
    };

    // Set up event callback to handle ingestion events
    let event_callback = Arc::new(move |event: IngestionEvent| {
        handle_ingestion_event(event, log_collector.clone(), debug_reporter.clone())
    });

    let service = service.with_event_callback(event_callback);

    // Start the service
    if let Err(e) = service.start().await {
        error!("Log processing failed: {}", e);
    }
}
