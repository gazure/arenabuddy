use std::{fmt::Display, path::PathBuf, sync::Arc, time::Duration};

use tokio::{
    sync::mpsc::{self},
    time::interval,
};
use tracing::{debug, error, info, warn};

use crate::{
    Error, Result,
    errors::ParseError,
    events::{business::BusinessEvent, draft::RequestTypeDraftNotify},
    models::MTGADraft,
    player_log::{
        draft::DraftBuilder,
        processor::{LogItem, ParseOutput, PlayerLogProcessor},
        replay::{MatchReplay, MatchReplayBuilder},
        watcher::LogWatcher,
    },
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
    /// Whether to use filesystem notifications. Following always checks for
    /// replacement and truncation by polling, even if notifications are disabled.
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
    /// The log file was replaced or truncated; unfinished event state was reset.
    LogRotated,
}

impl Display for IngestionEvent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IngestionEvent::DraftNotify(draft) => write!(f, "DraftNotify: {draft:?}"),
            IngestionEvent::Business(business) => write!(f, "Business: {business:?}"),
            IngestionEvent::MatchCompleted(match_replay) => write!(f, "MatchCompleted: {match_replay:?}"),
            IngestionEvent::ParseError(error) => write!(f, "ParseError: {error}"),
            IngestionEvent::LogRotated => write!(f, "LogRotated"),
        }
    }
}

/// Callback for handling ingestion events
pub type EventCallback =
    Arc<dyn Fn(IngestionEvent) -> std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> + Send + Sync>;

/// Service for ingesting and processing MTGA player logs
pub struct LogIngestionService {
    config: IngestionConfig,
    processor: PlayerLogProcessor,
    match_replay_builder: MatchReplayBuilder,
    draft_builder: DraftBuilder,
    event_callback: Option<EventCallback>,
    shutdown_rx: Option<mpsc::UnboundedReceiver<()>>,
    shutdown_on_ctrl_c: bool,
}

impl LogIngestionService {
    /// Create a new log ingestion service
    ///
    /// # Errors
    /// Returns an error for an unreadable log or a zero polling interval. In
    /// follow mode, a missing log is retried until the file appears.
    pub async fn new(config: IngestionConfig) -> Result<Self> {
        if config.poll_interval.is_zero() {
            return Err(Error::Io("poll interval must be greater than zero".into()));
        }
        let processor = PlayerLogProcessor::with_follow(&config.player_log_path, config.follow).await?;

        Ok(Self {
            config,
            processor,
            match_replay_builder: MatchReplayBuilder::new(),
            draft_builder: DraftBuilder::new(),
            event_callback: None,
            shutdown_rx: None,
            shutdown_on_ctrl_c: false,
        })
    }

    /// Add a replay writer
    #[must_use]
    pub fn add_writer(mut self, writer: Box<dyn ReplayWriter>) -> Self {
        self.match_replay_builder.add_writer(writer);
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

    /// Stops ingestion on Ctrl+C without installing a handler per service.
    #[must_use]
    pub fn with_shutdown(mut self) -> Self {
        self.shutdown_on_ctrl_c = true;
        self
    }

    /// Stops ingestion when the sender signals or drops the supplied receiver.
    #[must_use]
    pub fn with_shutdown_receiver(mut self, receiver: mpsc::UnboundedReceiver<()>) -> Self {
        self.shutdown_rx = Some(receiver);
        self
    }

    /// Emit an event to the callback if one is set
    async fn emit_event(&self, event: IngestionEvent) {
        if let Some(callback) = &self.event_callback {
            callback(event).await;
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
                self.emit_event(IngestionEvent::DraftNotify(event.clone())).await;
            }
            ParseOutput::BusinessMessage(event) => {
                self.emit_event(IngestionEvent::Business(Box::new(event.request.clone())))
                    .await;
                self.draft_builder.process_event(&event.request).await?;
            }
            _ => {}
        }

        // Process match replay
        match self.match_replay_builder.ingest(output).await {
            Ok(Some(match_replay)) => {
                self.emit_event(IngestionEvent::MatchCompleted(Box::new(match_replay)))
                    .await;
            }
            Err(e) => {
                error!("Error processing match replay: {}", e);
            }
            _ => {}
        }

        Ok(())
    }

    fn reset_builders(&mut self) {
        self.match_replay_builder.reset();
        self.draft_builder.reset();
    }

    /// Follows appended bytes and resets state only at confirmed file boundaries.
    ///
    /// # Errors
    ///
    /// Returns an error if log reads, signal handling, or draft processing fail.
    /// Notification setup failures fall back to polling.
    pub async fn start(mut self) -> Result<()> {
        let mut poll_interval = interval(self.config.poll_interval);
        poll_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        let mut watcher = if self.config.follow && self.config.watch_rotation {
            match LogWatcher::new(&self.config.player_log_path) {
                Ok(watcher) => Some(watcher),
                Err(error) => {
                    warn!("Log notifications unavailable; following by polling: {error}");
                    None
                }
            }
        } else {
            None
        };

        let mut shutdown_rx = self.shutdown_rx.take();
        let on_ctrl_c = self.shutdown_on_ctrl_c;
        let shutdown = async move {
            tokio::select! {
                result = async {
                    if on_ctrl_c { tokio::signal::ctrl_c().await }
                    else { std::future::pending::<std::io::Result<()>>().await }
                } => result,
                () = async {
                    if let Some(rx) = &mut shutdown_rx { rx.recv().await; }
                    else { std::future::pending::<()>().await; }
                } => Ok(()),
            }
        };
        tokio::pin!(shutdown);
        let mut at_eof = false;
        info!("Starting log ingestion from: {:?}", self.config.player_log_path);

        loop {
            if at_eof {
                tokio::select! {
                    biased;
                    result = &mut shutdown => { result?; break; }
                    _ = poll_interval.tick() => {}
                    notification = async {
                        if let Some(watcher) = &mut watcher { watcher.wakeups.recv().await }
                        else { std::future::pending::<Option<()>>().await }
                    } => {
                        if notification.is_none() { watcher = None; }
                    }
                }
            }
            let next = tokio::select! {
                biased;
                result = &mut shutdown => { result?; break; }
                next = self.processor.next_item() => next,
            };
            at_eof = false;
            match next {
                Ok(LogItem::Event(output)) => self.process_parse_output(output).await?,
                Ok(LogItem::Boundary) => {
                    self.reset_builders();
                    self.emit_event(IngestionEvent::LogRotated).await;
                }
                Ok(LogItem::Progress) => {}
                Ok(LogItem::Eof) if self.config.follow => at_eof = true,
                Ok(LogItem::Eof) => {
                    if let Err(Error::Parse(ParseError::Error(reason))) = self.processor.finish() {
                        self.emit_event(IngestionEvent::ParseError(reason)).await;
                    }
                    break;
                }
                Err(Error::Parse(ParseError::Error(reason))) => {
                    debug!("Parse error: {reason}");
                    self.emit_event(IngestionEvent::ParseError(reason)).await;
                }
                Err(error) => return Err(error),
            }
            tokio::task::yield_now().await;
        }
        Ok(())
    }
}

#[cfg(test)]
#[path = "ingest_tests.rs"]
mod tests;
