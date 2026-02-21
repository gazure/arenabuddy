use std::sync::Arc;

use arenabuddy_core::{
    cards::CardsDatabase,
    services::{
        auth_service::auth_service_server::AuthServiceServer, debug_service::debug_service_server::DebugServiceServer,
        match_service::match_service_server::MatchServiceServer,
    },
};
use arenabuddy_data::{ArenabuddyRepository, MatchDB};
use tonic::transport::Server;
use tracingx::info;

use crate::{
    auth::{AuthConfig, AuthServiceImpl, auth_interceptor},
    debug_service::DebugServiceImpl,
    match_service::MatchServiceImpl,
};

pub mod auth;
mod debug_service;
mod match_service;
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
    tracingx::init_compact();

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
        .serve(addr)
        .await?;

    Ok(())
}
