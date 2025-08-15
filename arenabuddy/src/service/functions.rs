use arenabuddy_core::{display::match_details::MatchDetails, models::MTGAMatch};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use crate::service::Service;

// Convenience functions that mirror the original Tauri commands
#[server]
pub async fn command_matches() -> ServerFnResult<Vec<MTGAMatch>> {
    let FromContext(service): FromContext<Service> = extract().await?;
    service
        .get_matches()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_match_details(id: String) -> ServerFnResult<MatchDetails> {
    let FromContext(service): FromContext<Service> = extract().await?;
    service
        .get_match_details(id)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_error_logs() -> ServerFnResult<Vec<String>> {
    let FromContext(service): FromContext<Service> = extract().await?;
    service
        .get_error_logs()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_set_debug_logs(path: String) -> ServerFnResult<()> {
    let FromContext(service): FromContext<Service> = extract().await?;
    service
        .set_debug_logs(path)
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}

#[server]
pub async fn command_get_debug_logs() -> ServerFnResult<Option<Vec<String>>> {
    let FromContext(service): FromContext<Service> = extract().await?;
    service
        .get_debug_logs()
        .await
        .map_err(|e| ServerFnError::ServerError(e.to_string()))
}
