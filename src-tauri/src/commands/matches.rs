use arenabuddy_core::match_insights::MatchDB;
use arenabuddy_core::models::mtga_match::MTGAMatch;
use std::sync::{Arc, Mutex};
use tauri::State;
use tracing::error;

#[tauri::command]
pub fn command_matches(db: State<'_, Arc<Mutex<MatchDB>>>) -> Vec<MTGAMatch> {
    let mut db = db.inner().lock().expect("Failed to lock db");
    db.get_matches()
        .unwrap_or_else(|e| {
            error!("error retrieving matches {}", e);
            Vec::default()
        })
        .into_iter()
        .rev()
        .collect()
}
