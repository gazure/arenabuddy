use std::sync::Arc;

use arenabuddy_core::models::MTGAMatch;
use arenabuddy_data::MatchDB;
use tauri::State;
use tokio::sync::Mutex;
use tracing::error;

#[tauri::command]
pub async fn command_matches(db: State<'_, Arc<Mutex<MatchDB>>>) -> Result<Vec<MTGAMatch>, ()> {
    let db = db.inner().lock().await;
    Ok(db
        .get_matches()
        .await
        .unwrap_or_else(|e| {
            error!("error retrieving matches {}", e);
            Vec::default()
        })
        .into_iter()
        .rev()
        .collect())
}
