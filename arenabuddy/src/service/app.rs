use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use arenabuddy_core::{
    cards::CardsDatabase,
    display::{
        deck::{DeckDisplayRecord, Difference},
        game::GameResultDisplay,
        match_details::MatchDetails,
        mulligan::Mulligan,
    },
    models::MTGAMatch,
};
use arenabuddy_data::{DirectoryStorage, MatchDB};
use dioxus::{
    LaunchBuilder,
    desktop::{Config, WindowBuilder},
};
use start::AppMeta;
use tokio::sync::Mutex;
use tracing::{Level, debug, error, info};
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracing_subscriber::{
    EnvFilter, Layer,
    fmt::{self, writer::MakeWriterExt},
    layer::SubscriberExt,
    util::SubscriberInitExt,
};

use crate::{Error, Result, app::App};

#[derive(Clone)]
pub struct AppService<D: arenabuddy_data::ArenabuddyRepository> {
    pub db: Arc<Mutex<D>>,
    pub cards: Arc<CardsDatabase>,
    pub log_collector: Arc<Mutex<Vec<String>>>,
    pub debug_storage: Arc<Mutex<Option<DirectoryStorage>>>,
}
pub type Service = AppService<MatchDB>;

impl<D> std::fmt::Debug for AppService<D>
where
    D: arenabuddy_data::ArenabuddyRepository,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AppService")
            .field("db", &"Arc<Mutex<MatchDB>>")
            .field("cards", &"CardsDatabase")
            .field("log_collector", &"Arc<Mutex<Vec<String>>>")
            .field("debug_backend", &"Arc<Mutex<Option<DirectoryStorage>>>")
            .finish()
    }
}

impl<D> AppService<D>
where
    D: arenabuddy_data::ArenabuddyRepository,
{
    pub fn new(
        db: Arc<Mutex<D>>,
        cards: Arc<CardsDatabase>,
        log_collector: Arc<Mutex<Vec<String>>>,
        debug_backend: Arc<Mutex<Option<DirectoryStorage>>>,
    ) -> Self {
        Self {
            db,
            cards,
            log_collector,
            debug_storage: debug_backend,
        }
    }

    pub async fn get_matches(&self) -> Result<Vec<MTGAMatch>> {
        let mut db = self.db.lock().await;
        Ok(db.list_matches().await?)
    }

    pub async fn get_match_details(&self, id: String) -> Result<MatchDetails> {
        let mut db = self.db.lock().await;
        info!("looking for match {id}");

        let (mtga_match, result) = db.get_match(&id).await.unwrap_or_default();

        let mut match_details = MatchDetails {
            id: id.clone(),
            controller_seat_id: mtga_match.controller_seat_id(),
            controller_player_name: mtga_match.controller_player_name().to_string(),
            opponent_player_name: mtga_match.opponent_player_name().to_string(),
            created_at: mtga_match.created_at(),
            did_controller_win: result.is_some_and(|r| r.is_winner(mtga_match.controller_seat_id())),
            ..Default::default()
        };

        match_details.decklists = db.list_decklists(&id).await.unwrap_or_default();

        match_details.primary_decklist = match_details
            .decklists
            .first()
            .map(|primary_decklist| DeckDisplayRecord::from_decklist(primary_decklist, &self.cards));

        match_details.decklists.windows(2).for_each(|pair| {
            if let [prev, next] = pair {
                let diff = Difference::diff(prev, next, &self.cards);
                match_details.differences.get_or_insert_with(Vec::new).push(diff);
            }
        });

        let raw_mulligans = db.list_mulligans(&id).await.unwrap_or_else(|e| {
            error!("Error retrieving Mulligans: {}", e);
            Vec::default()
        });

        match_details.mulligans = raw_mulligans
            .iter()
            .map(|mulligan| Mulligan::from_model(mulligan, &self.cards))
            .collect();

        match_details.mulligans.sort();

        match_details.game_results = db
            .list_match_results(&id)
            .await
            .unwrap_or_else(|e| {
                error!("Error retrieving game results: {}", e);
                Vec::default()
            })
            .iter()
            .map(|mr| {
                GameResultDisplay::from_match_result(
                    mr,
                    match_details.controller_seat_id,
                    &match_details.controller_player_name,
                    &match_details.opponent_player_name,
                )
            })
            .collect();

        match_details.opponent_deck = db
            .get_opponent_deck(&id)
            .await
            .map(|deck| DeckDisplayRecord::from_decklist(&deck, &self.cards))
            .ok();

        Ok(match_details)
    }

    pub async fn get_error_logs(&self) -> Result<Vec<String>> {
        let logs = self.log_collector.lock().await;
        Ok(logs.clone())
    }

    pub async fn set_debug_logs(&self, path: String) -> Result<()> {
        let storage = DirectoryStorage::new(path.into());
        let mut debug_backend = self.debug_storage.lock().await;
        *debug_backend = Some(storage);
        Ok(())
    }

    pub async fn get_debug_logs(&self) -> Result<Option<Vec<String>>> {
        let debug_backend = self.debug_storage.lock().await;
        if let Some(_storage) = &*debug_backend {
            // Implementation depends on DirectoryStorage interface
            // This is a placeholder - adjust based on actual interface
            Ok(Some(vec!["Debug logs not yet implemented".to_string()]))
        } else {
            Ok(None)
        }
    }
}

pub fn launch_app() -> Result<()> {
    let data_dir = get_app_data_dir()?;
    let resource_dir = get_resource_dir()?;
    let home = std::env::home_dir().ok_or(Error::NoHomeDir)?;
    let player_log_path = match std::env::consts::OS {
        "macos" => Ok(home.join("Library/Logs/Wizards of the Coast/MTGA/Player.log")),
        "windows" => Ok(home.join("AppData/LocalLow/Wizards of the Coast/MTGA/Player.log")),
        _ => Err(Error::UnsupportedOS),
    }?;
    info!("Processing logs from : {}", player_log_path.to_string_lossy());

    let background = tokio::runtime::Runtime::new()?;
    let service = background.block_on(create_app_service())?;
    let service2 = service.clone();
    background.spawn(async move {
        crate::service::ingest::start(
            service2.db.clone(),
            service2.debug_storage.clone(),
            service2.log_collector.clone(),
            player_log_path,
        )
        .await;
    });

    LaunchBuilder::desktop()
        .with_cfg(
            Config::new()
                .with_data_directory(data_dir.clone())
                .with_resource_directory(resource_dir.clone())
                .with_window(WindowBuilder::new().with_resizable(true)),
        )
        .with_context(service)
        .launch(App);
    Ok(())
}

fn get_app_data_dir() -> Result<std::path::PathBuf> {
    let home = std::env::home_dir().ok_or(Error::NoHomeDir)?;

    let app_data = match std::env::consts::OS {
        "macos" => home.join("Library/Application Support/com.gazure.dev.arenabuddy.app"),
        "windows" => home.join("AppData/Roaming/com.gazure.dev.arenabuddy.app"),
        "linux" => home.join(".local/share/com.gazure.dev.arenabuddy.app"),
        _ => return Err(Error::UnsupportedOS),
    };

    std::fs::create_dir_all(&app_data).map_err(|_| Error::CorruptedAppData)?;
    Ok(app_data)
}

fn get_resource_dir() -> Result<std::path::PathBuf> {
    // In a desktop app, resources are typically bundled with the executable
    let exe_dir = std::env::current_exe()?.parent().unwrap().to_path_buf();

    // Check common resource locations
    let possible_paths = vec![
        exe_dir.join("resources"),
        exe_dir.join("data"),
        exe_dir.parent().unwrap().join("Resources"), // macOS app bundle
        std::env::var("CARGO_MANIFEST_DIR").map_or_else(
            |_| std::env::current_dir().unwrap().join("./data"),
            |dir| std::path::PathBuf::from(dir).join("data"),
        ), // Development mode
    ];

    for path in possible_paths {
        debug!("looking in: {:?}", path);
        if path.join("cards-full.pb").exists() {
            return Ok(path);
        }
    }

    Err(Error::NoCardsDatabase)
}

fn setup_logging(app_data_dir: &Path) -> Result<()> {
    let registry = tracing_subscriber::registry();
    let log_dir = app_data_dir.join("logs");
    std::fs::create_dir_all(&log_dir).map_err(|_| Error::CorruptedAppData)?;

    let file_appender = RollingFileAppender::builder()
        .rotation(Rotation::DAILY)
        .filename_prefix("arena-buddy")
        .build(log_dir)
        .map_err(|_| Error::LogFailure)?
        .with_max_level(Level::INFO);

    let file_layer = fmt::layer()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_target(false)
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    let console_layer = fmt::Layer::new()
        .with_target(true)
        .with_line_number(true)
        .with_file(true)
        .with_level(true);

    let console_filter = EnvFilter::new("info");

    let registry = registry
        .with(file_layer)
        .with(console_layer.with_filter(console_filter));

    #[cfg(feature = "debug")]
    {
        let (console_layer, server) = console_subscriber::ConsoleLayer::builder().with_default_env().build();
        tokio::spawn(async { server.serve().await });
        registry
            .with(console_layer.with_filter(EnvFilter::new("tokio=trace,runtime=trace")))
            .init();
    }
    #[cfg(not(feature = "debug"))]
    {
        registry.init();
    }

    Ok(())
}

async fn create_app_service() -> Result<Service> {
    let data_dir = get_app_data_dir()?;
    let resource_dir = get_resource_dir()?;
    setup_logging(&data_dir)?;

    build_service(data_dir, resource_dir).await
}

async fn build_service(_app_data_dir: PathBuf, resource_dir: PathBuf) -> Result<Service> {
    let app_meta = AppMeta::from_env().with_app_name("arenabuddy");
    let root_span = tracing::info_span!("app", app = %app_meta.app);
    let _span = root_span.enter();

    info!("resource dir: {:?}", resource_dir);
    let cards_path = resource_dir.join("cards-full.pb");
    info!("cards_db path: {:?}", cards_path);
    let cards_db = Arc::new(CardsDatabase::new(cards_path).map_err(|_| Error::NoCardsDatabase)?);

    let url = std::env::var("ARENABUDDY_DATABASE_URL").ok();
    info!("using matches db: {:?}", url);
    let mut db = MatchDB::new(url.as_deref(), cards_db.clone()).await?;
    db.initialize().await?;
    let db_arc = Arc::new(tokio::sync::Mutex::new(db));

    let log_collector = Arc::new(tokio::sync::Mutex::new(Vec::<String>::new()));
    let debug_backend = Arc::new(tokio::sync::Mutex::new(None::<DirectoryStorage>));

    Ok(AppService::new(
        db_arc.clone(),
        cards_db.clone(),
        log_collector,
        debug_backend,
    ))
}
