use std::sync::{Arc, Mutex};

use tauri::State;

#[tauri::command]
pub fn command_error_logs(log_collector: State<'_, Arc<Mutex<Vec<String>>>>) -> Vec<String> {
    let lc = log_collector.lock().expect("log collector lock poisoned");
    lc.clone()
}
