use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use tracing::error;

/// UI color theme.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Theme {
    Light,
    #[default]
    Dark,
}

impl Theme {
    pub fn toggled(self) -> Self {
        match self {
            Theme::Light => Theme::Dark,
            Theme::Dark => Theme::Light,
        }
    }

    pub fn is_dark(self) -> bool {
        self == Theme::Dark
    }
}

fn theme_file_path() -> Option<PathBuf> {
    Some(super::paths::app_data_dir()?.join("theme.json"))
}

/// Load the saved theme from disk, defaulting to dark if missing or malformed.
pub fn load_theme() -> Theme {
    let Some(path) = theme_file_path() else {
        return Theme::default();
    };
    std::fs::read_to_string(&path)
        .ok()
        .and_then(|json| serde_json::from_str(&json).ok())
        .unwrap_or_default()
}

/// Save the theme choice to disk.
pub fn save_theme(theme: Theme) {
    let Some(path) = theme_file_path() else {
        return;
    };
    match serde_json::to_string(&theme) {
        Ok(json) => {
            if let Err(e) = std::fs::write(&path, json) {
                error!("Failed to save theme: {e}");
            }
        }
        Err(e) => error!("Failed to serialize theme: {e}"),
    }
}
