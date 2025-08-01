use std::sync::Arc;

use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn command_error_logs(
    log_collector: State<'_, Arc<Mutex<Vec<String>>>>,
) -> Result<Vec<String>, ()> {
    let lc = log_collector.lock().await;
    Ok(lc.clone())
}
