use std::{path::Path, sync::Arc};

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{DirectoryStorage, MatchDB};
use dioxus::{
    LaunchBuilder,
    desktop::{Config, WindowBuilder},
};
use start::AppMeta;
use tracing_appender::rolling::{RollingFileAppender, Rotation};
use tracingx::{
    EnvFilter, Layer, Level, SubscriberExt, SubscriberInitExt, debug,
    fmt::{self, writer::MakeWriterExt},
    info,
};

use crate::{
    Error, Result,
    app::App,
    backend::{Service, service::AppService},
};

pub fn launch() -> Result<()> {
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
        crate::backend::ingest::start(
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
                .with_window(WindowBuilder::new().with_title("Arenabuddy").with_resizable(true)),
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
    let registry = tracingx::registry();
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

    let app_meta = AppMeta::from_env().with_app_name("arenabuddy");
    let root_span = tracingx::info_span!("app", app = %app_meta.app);
    let _span = root_span.enter();
    info!("resource dir: {:?}", resource_dir);
    let cards_path = resource_dir.join("cards-full.pb");
    info!("cards_db path: {:?}", cards_path);
    let cards_db = Arc::new(CardsDatabase::new(cards_path).unwrap_or_default());
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
