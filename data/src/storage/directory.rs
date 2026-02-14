use std::path::PathBuf;

use arenabuddy_core::{
    Result,
    player_log::{ingest::ReplayWriter, replay::MatchReplay},
};
use async_trait::async_trait;
use tokio::fs::File;
use tracingx::info;

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

    pub async fn list_replays(&self) -> Result<Vec<String>> {
        let mut entries = tokio::fs::read_dir(&self.path).await?;
        let mut replays = Vec::new();
        while let Some(entry) = entries.next_entry().await? {
            if let Some(name) = entry.file_name().to_str()
                && std::path::Path::new(name)
                    .extension()
                    .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
            {
                replays.push(name.to_string());
            }
        }
        replays.sort();
        Ok(replays)
    }
}

#[async_trait]
impl ReplayWriter for DirectoryStorage {
    async fn write(&mut self, match_replay: &MatchReplay) -> Result<()> {
        let path = self.path.join(format!("{}.json", match_replay.match_id));
        info!(
            "Writing match replay to file: {}",
            path.to_str().unwrap_or("Path not found")
        );
        let file = File::create(&path).await?;
        let mut writer = tokio::io::BufWriter::new(file);

        tokio::io::AsyncWriteExt::write_all(&mut writer, &serde_json::to_vec_pretty(match_replay)?).await?;

        info!("Match replay written to file");
        Ok(())
    }
}
