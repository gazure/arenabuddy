use std::{fs::File, io::BufWriter, path::PathBuf};

use arenabuddy_core::replay::MatchReplay;
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
    fn write(&mut self, match_replay: &MatchReplay) -> Result<()> {
        let path = self.path.join(format!("{}.json", match_replay.match_id));
        info!(
            "Writing match replay to file: {}",
            path.clone().to_str().unwrap_or("Path not found")
        );
        let file = File::create(path)?;
        let writer = BufWriter::new(file);

        serde_json::to_writer(writer, match_replay)?;

        info!("Match replay written to file");
        Ok(())
    }
}
