use std::{
    path::{Path, PathBuf},
    sync::Arc,
    time::Duration,
};

use tokio::{
    sync::mpsc::{self},
    time::interval,
};
use tracingx::{debug, error, info};

use crate::{
    Error, Result,
    draft::DraftBuilder,
    errors::ParseError,
    models::MTGADraft,
    mtga_events::{business::BusinessEvent, draft::RequestTypeDraftNotify},
    processor::{ParseOutput, PlayerLogProcessor},
    replay::{MatchReplay, MatchReplayBuilder},
};

/// Storage trait for writing match replays
#[async_trait::async_trait]
pub trait ReplayWriter: Send + Sync {
    async fn write(&mut self, replay: &MatchReplay) -> Result<()>;
}

/// Storage trait for writing draft pod results
#[async_trait::async_trait]
pub trait DraftWriter: Send + Sync {
    async fn write(&mut self, draft: &MTGADraft) -> Result<()>;
}

/// Configuration for the log ingestion service
#[derive(Debug, Clone)]
pub struct IngestionConfig {
    /// Path to the player log file
    pub player_log_path: PathBuf,
    /// Whether to continuously follow the log file
    pub follow: bool,
    /// Interval between processing attempts
    pub poll_interval: Duration,
    /// Whether to watch for log file rotation
    pub watch_rotation: bool,
}

impl IngestionConfig {
    pub fn new(player_log_path: PathBuf) -> Self {
        Self {
            player_log_path,
            follow: true,
            poll_interval: Duration::from_secs(1),
            watch_rotation: true,
        }
    }

    #[must_use]
    pub fn with_follow(mut self, follow: bool) -> Self {
        self.follow = follow;
        self
    }

    #[must_use]
    pub fn with_poll_interval(mut self, interval: Duration) -> Self {
        self.poll_interval = interval;
        self
    }

    #[must_use]
    pub fn with_rotation_watch(mut self, watch: bool) -> Self {
        self.watch_rotation = watch;
        self
    }
}

/// Events that can be emitted during log ingestion
#[derive(Debug)]
pub enum IngestionEvent {
    /// A draft notify event was found
    DraftNotify(RequestTypeDraftNotify),
    /// An MTGA Business Event
    Business(Box<BusinessEvent>),
    /// A match replay was completed
    MatchCompleted(Box<MatchReplay>),
    /// An error occurred while parsing
    ParseError(String),
    /// The log file was rotated
    LogRotated,
}

/// Callback for handling ingestion events
pub type EventCallback = Arc<dyn Fn(IngestionEvent) + Send + Sync>;

/// Service for ingesting and processing MTGA player logs
pub struct LogIngestionService {
    config: IngestionConfig,
    processor: PlayerLogProcessor,
    match_replay_builder: MatchReplayBuilder,
    draft_builder: DraftBuilder,
    writers: Vec<Box<dyn ReplayWriter>>,
    event_callback: Option<EventCallback>,
    shutdown_rx: Option<mpsc::UnboundedReceiver<()>>,
}

impl LogIngestionService {
    /// Create a new log ingestion service
    ///
    /// # Errors
    /// Errors if Player log cannot be found
    pub async fn new(config: IngestionConfig) -> Result<Self> {
        let processor = PlayerLogProcessor::try_new(&config.player_log_path).await?;

        Ok(Self {
            config,
            processor,
            match_replay_builder: MatchReplayBuilder::new(),
            draft_builder: DraftBuilder::new(),
            writers: Vec::new(),
            event_callback: None,
            shutdown_rx: None,
        })
    }

    /// Add a replay writer
    #[must_use]
    pub fn add_writer(mut self, writer: Box<dyn ReplayWriter>) -> Self {
        self.writers.push(writer);
        self
    }

    /// Add a draft writer
    #[must_use]
    pub fn add_draft_writer(mut self, writer: Box<dyn DraftWriter>) -> Self {
        self.draft_builder.add_writer(writer);
        self
    }

    /// Set an event callback for handling ingestion events
    #[must_use]
    pub fn with_event_callback(mut self, callback: EventCallback) -> Self {
        self.event_callback = Some(callback);
        self
    }

    /// Set a shutdown receiver for graceful termination
    #[must_use]
    pub fn with_shutdown(mut self) -> Self {
        self.shutdown_rx = Some(create_shutdown_channel());
        self
    }

    /// Emit an event to the callback if one is set
    fn emit_event(&self, event: IngestionEvent) {
        if let Some(callback) = &self.event_callback {
            callback(event);
        }
    }

    /// Process a single parse output
    ///
    /// # Errors
    /// Returns an error if the event could not be processed.
    async fn process_parse_output(&mut self, output: ParseOutput) -> Result<()> {
        // Emit draft events
        match &output {
            ParseOutput::DraftNotify(event) => {
                self.emit_event(IngestionEvent::DraftNotify(event.clone()));
            }
            ParseOutput::BusinessMessage(event) => {
                self.emit_event(IngestionEvent::Business(Box::new(event.request.clone())));
                self.draft_builder.process_business_event(&event.request).await?;
            }
            _ => {}
        }

        // Process match replay
        if self.match_replay_builder.ingest(output) {
            let builder = std::mem::replace(&mut self.match_replay_builder, MatchReplayBuilder::new());
            match builder.build() {
                Ok(match_replay) => {
                    // Write to all configured writers
                    for writer in &mut self.writers {
                        if let Err(e) = writer.write(&match_replay).await {
                            error!("Error writing match replay: {}", e);
                        }
                    }
                    self.emit_event(IngestionEvent::MatchCompleted(Box::new(match_replay)));
                }
                Err(e) => {
                    error!("Error building match replay: {}", e);
                }
            }
        }

        Ok(())
    }

    /// Process all available events from the log
    async fn process_available_events(&mut self) -> Result<bool> {
        let mut has_events = false;

        loop {
            match self.processor.get_next_event().await {
                Ok(output) => {
                    has_events = true;
                    self.process_parse_output(output).await?;
                }
                Err(e) => match e {
                    Error::Parse(ParseError::NoEvent) => break,
                    Error::Parse(ParseError::Error(s)) => {
                        debug!("Parse error: {}", s);
                        self.emit_event(IngestionEvent::ParseError(s));
                    }
                    _ => return Err(e),
                },
            }
        }

        Ok(has_events)
    }

    /// Handle log file rotation
    async fn handle_rotation(&mut self) -> Result<()> {
        info!("Log file rotated, reinitializing processor");
        self.processor = PlayerLogProcessor::try_new(&self.config.player_log_path).await?;
        self.emit_event(IngestionEvent::LogRotated);
        Ok(())
    }

    /// Start the ingestion service
    ///
    /// # Errors
    /// Errors if rotation watching is enabled and cannot be configured
    pub async fn start(mut self) -> Result<()> {
        let mut poll_interval = interval(self.config.poll_interval);
        poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

        // Set up file watcher if needed
        let mut rotation_rx = if self.config.watch_rotation {
            Some(Self::create_rotation_watcher(&self.config.player_log_path)?)
        } else {
            None
        };

        info!("Starting log ingestion from: {:?}", self.config.player_log_path);

        loop {
            tokio::select! {
                // Handle shutdown signal
                () = async {
                    if let Some(rx) = &mut self.shutdown_rx {
                        rx.recv().await;
                    } else {
                        std::future::pending::<()>().await;
                    }
                } => {
                    info!("Received shutdown signal");
                    break;
                }

                // Handle log rotation
                _ = async {
                    if let Some(rx) = &mut rotation_rx {
                        rx.recv().await
                    } else {
                        std::future::pending::<Option<()>>().await
                    }
                } => {
                    self.handle_rotation().await?;
                }

                // Process events on interval
                _ = poll_interval.tick() => {
                    let has_events = self.process_available_events().await?;

                    // If not following and no events, we're done
                    if !self.config.follow && !has_events {
                        info!("Finished processing log (follow=false)");
                        break;
                    }
                }
            }
        }

        Ok(())
    }

    /// Create a file watcher for log rotation detection
    ///
    /// # Errors
    /// Errors if FS notifier cannot be created
    fn create_rotation_watcher(path: &Path) -> Result<mpsc::UnboundedReceiver<()>> {
        use notify::{Event, EventKind, RecursiveMode, Watcher};

        let (tx, rx) = mpsc::unbounded_channel();
        let _watched_path = path.to_path_buf();

        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                // Check if this is a modification or create event that indicates rotation
                if matches!(
                    event.kind,
                    EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                ) {
                    let _ = tx.send(());
                }
            }
        })
        .map_err(|e| crate::Error::Io(e.to_string()))?;

        watcher
            .watch(path.parent().unwrap_or(Path::new(".")), RecursiveMode::NonRecursive)
            .map_err(|e| crate::Error::Io(e.to_string()))?;

        // Keep the watcher alive by leaking it (it will be cleaned up on process exit)
        Box::leak(Box::new(watcher));

        Ok(rx)
    }
}

/// Create a shutdown channel for graceful termination
fn create_shutdown_channel() -> mpsc::UnboundedReceiver<()> {
    let (tx, rx) = mpsc::unbounded_channel();

    ctrlc::set_handler(move || {
        let _ = tx.send(());
    })
    .expect("Could not set up SIGINT handler with system");

    rx
}
