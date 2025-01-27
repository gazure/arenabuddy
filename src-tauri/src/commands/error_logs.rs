use crate::log_collector::LogCollector;
use std::sync::{Arc, Mutex};
use tauri::State;

#[tauri::command]
pub(crate) fn command_error_logs(
    log_collector: State<'_, Arc<Mutex<LogCollector>>>,
) -> Vec<String> {
    let lock = log_collector.lock().expect("log collector lock poisoned");
    lock.get().clone()
}
