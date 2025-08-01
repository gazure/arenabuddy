use std::path::PathBuf;

use arenabuddy_core::replay::MatchReplay;
use tokio::fs::File;
use tracing::info;

use crate::{Result, Storage};

pub struct DirectoryStorage {
    path: PathBuf,
}

impl DirectoryStorage {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Storage for DirectoryStorage {
    async fn write(&mut self, match_replay: &MatchReplay) -> Result<()> {
        let path = self.path.join(format!("{}.json", match_replay.match_id));
        info!(
            "Writing match replay to file: {}",
            path.to_str().unwrap_or("Path not found")
        );
        let file = File::create(&path).await?;
        let mut writer = tokio::io::BufWriter::new(file);

        tokio::io::AsyncWriteExt::write_all(&mut writer, &serde_json::to_vec_pretty(match_replay)?)
            .await?;

        info!("Match replay written to file");
        Ok(())
    }
}
