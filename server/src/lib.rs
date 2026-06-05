use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    services::{
        auth_service::auth_service_server::AuthServiceServer, debug_service::debug_service_server::DebugServiceServer,
        match_service::match_service_server::MatchServiceServer,
    },
};
use arenabuddy_data::{ArenabuddyRepository, CardRepository, MatchDB};
use tonic::transport::Server;
use tracing::info;

use crate::{
    auth::{AuthConfig, AuthServiceImpl, auth_interceptor},
    debug_service::DebugServiceImpl,
    match_service::MatchServiceImpl,
};

pub mod auth;
mod debug_service;
mod match_service;
#[cfg(feature = "otel")]
mod otel;
mod sheets_sync;

/// Start the gRPC server with all services.
///
/// # Errors
/// Returns an error if database initialization fails or the server cannot
/// bind to the listen address.
///
/// # Panics
/// Panics if required environment variables are missing: `DATABASE_URL`,
/// `DISCORD_CLIENT_ID`, `DISCORD_CLIENT_SECRET`, or `JWT_SECRET`.
pub async fn run() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(feature = "otel")]
    let otel_guard = otel::init_compact_with_otel("arenabuddy-server");
    #[cfg(not(feature = "otel"))]
    {
        use tracing_subscriber::{EnvFilter, Registry, fmt, layer::SubscriberExt, util::SubscriberInitExt};
        let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
        let fmt_layer = fmt::layer()
            .compact()
            .with_target(true)
            .with_level(true)
            .with_thread_names(false)
            .with_file(false)
            .with_line_number(false);
        Registry::default().with(env_filter).with(fmt_layer).init();
    }

    #[cfg(feature = "otel")]
    {
        let endpoint =
            std::env::var("OTEL_EXPORTER_OTLP_ENDPOINT").unwrap_or_else(|_| "http://localhost:4317".to_string());
        info!("OpenTelemetry enabled, exporting traces to {endpoint}");
    }
    #[cfg(not(feature = "otel"))]
    info!("OpenTelemetry disabled (compile with --features otel to enable)");

    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL environment variable must be set");

    let addr = std::env::var("LISTEN_ADDR")
        .unwrap_or_else(|_| "[::1]:50051".to_string())
        .parse()?;

    let auth_config = Arc::new(AuthConfig {
        discord_client_id: std::env::var("DISCORD_CLIENT_ID")
            .expect("DISCORD_CLIENT_ID environment variable must be set"),
        discord_client_secret: std::env::var("DISCORD_CLIENT_SECRET")
            .expect("DISCORD_CLIENT_SECRET environment variable must be set"),
        jwt_secret: std::env::var("JWT_SECRET").expect("JWT_SECRET environment variable must be set"),
    });

    info!("Connecting to database...");
    let cards = CardsDatabase::default();
    let db = MatchDB::new(Some(&database_url), cards.clone()).await?;
    db.init().await?;
    info!("Database initialized");

    load_cards_on_startup(&db, &cards).await?;

    let spreadsheet_id = std::env::var("GOOGLE_SHEETS_SPREADSHEET_ID").ok();
    if spreadsheet_id.is_some() {
        info!("Google Sheets sync enabled");
    }

    let match_service = MatchServiceImpl {
        db: db.clone(),
        cards: cards.clone(),
        spreadsheet_id,
    };
    let debug_service = DebugServiceImpl { db: db.clone() };
    let auth_service = AuthServiceImpl::new(db, auth_config.clone());

    let interceptor = auth_interceptor(auth_config.jwt_secret.clone());

    info!("Starting gRPC server on {addr}");
    Server::builder()
        .add_service(MatchServiceServer::with_interceptor(match_service, interceptor.clone()))
        .add_service(DebugServiceServer::with_interceptor(debug_service, interceptor))
        .add_service(AuthServiceServer::new(auth_service))
        .serve_with_shutdown(addr, async {
            tokio::signal::ctrl_c().await.ok();
            info!("Received shutdown signal");
        })
        .await?;

    #[cfg(feature = "otel")]
    {
        info!("Flushing OpenTelemetry traces...");
        otel_guard.shutdown();
    }

    Ok(())
}

/// Populate the `card` table from the embedded cards database.
///
/// By default this only loads when the table is empty, so normal restarts are
/// cheap. Set `ARENABUDDY_RELOAD_CARDS=1` (or `true`) to force a full reload
/// (TRUNCATE + reinsert) on startup, e.g. after shipping an updated
/// `cards-full.pb`.
async fn load_cards_on_startup(db: &MatchDB, cards: &CardsDatabase) -> Result<(), Box<dyn std::error::Error>> {
    let force_reload =
        std::env::var("ARENABUDDY_RELOAD_CARDS").is_ok_and(|v| matches!(v.trim(), "1" | "true" | "TRUE" | "yes"));

    let existing = db.card_count().await?;
    if existing > 0 && !force_reload {
        info!(
            "card table already populated ({existing} cards), skipping load (set ARENABUDDY_RELOAD_CARDS=1 to force)"
        );
        return Ok(());
    }

    let to_load: Vec<_> = cards.values().cloned().collect();
    info!(
        "Loading {} cards into Postgres (existing: {existing}, force_reload: {force_reload})",
        to_load.len()
    );
    db.load_cards(&to_load).await?;
    info!("Card load complete");
    Ok(())
}
