use std::{result::Result, sync::Arc};

use arenabuddy_data::DirectoryStorage;
use tauri::State;
use tokio::sync::Mutex;

#[tauri::command]
pub async fn command_set_debug_logs(
    dir: String,
    dir_backend: State<'_, Arc<Mutex<Option<DirectoryStorage>>>>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(dir);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let mut backend = dir_backend.lock().await;
    *backend = Some(DirectoryStorage::new(path));

    Ok(())
}

#[tauri::command]
pub async fn command_get_debug_logs(
    dir_backend: State<'_, Arc<Mutex<Option<DirectoryStorage>>>>,
) -> Result<Option<String>, ()> {
    Ok(dir_backend
        .lock()
        .await
        .as_ref()
        .map(|b| b.path().to_string_lossy().to_string()))
}
