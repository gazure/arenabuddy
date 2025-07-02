use std::{fs::File, io::BufWriter, path::PathBuf};

use tracing::info;

use crate::{Result, replay::MatchReplay};

pub trait Storage {
    /// # Errors
    ///
    /// Will return an error if the match replay cannot be written to the storage backend
    fn write(&mut self, match_replay: &MatchReplay) -> crate::Result<()>;
}

pub struct DirectoryStorageBackend {
    path: PathBuf,
}

impl DirectoryStorageBackend {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl Storage for DirectoryStorageBackend {
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
