use std::{
    result::Result,
    sync::{Arc, Mutex},
};

use arenabuddy_data::DirectoryStorage;
use tauri::State;

#[tauri::command]
pub fn command_set_debug_logs(
    dir: String,
    dir_backend: State<'_, Arc<Mutex<Option<DirectoryStorage>>>>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(dir);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let mut backend = dir_backend
        .lock()
        .expect("Failed to lock directory backend");
    *backend = Some(DirectoryStorage::new(path));

    Ok(())
}

#[tauri::command]
pub fn command_get_debug_logs(
    dir_backend: State<'_, Arc<Mutex<Option<DirectoryStorage>>>>,
) -> Option<String> {
    dir_backend
        .lock()
        .expect("Failed to lock directory backend")
        .as_ref()
        .map(|b| b.path().to_string_lossy().to_string())
}
