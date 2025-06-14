use std::{
    result::Result,
    sync::{Arc, Mutex},
};

use arenabuddy_core::storage::DirectoryStorageBackend;
use tauri::State;

#[tauri::command]
pub fn command_set_debug_logs(
    dir: String,
    dir_backend: State<'_, Arc<Mutex<Option<DirectoryStorageBackend>>>>,
) -> Result<(), String> {
    let path = std::path::PathBuf::from(dir);

    if !path.exists() {
        return Err(format!("Path does not exist: {}", path.display()));
    }

    let mut backend = dir_backend
        .lock()
        .expect("Failed to lock directory backend");
    *backend = Some(DirectoryStorageBackend::new(path));

    Ok(())
}
