use crate::replay::MatchReplay;
use anyhow::Result;
use serde::Serialize;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use tracing::info;

pub trait Storage {
    /// # Errors
    ///
    /// Will return an error if the match replay cannot be written to the storage backend
    fn write(&mut self, match_replay: &MatchReplay) -> anyhow::Result<()>;
}

pub struct DirectoryStorageBackend {
    path: PathBuf,
}

impl DirectoryStorageBackend {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }
}

fn write_line<T>(writer: &mut BufWriter<File>, line: &T) -> Result<()>
where
    T: Serialize,
{
    let line_str = serde_json::to_string(line)?;
    writer.write_all(line_str.as_bytes())?;
    writer.write_all(b"\n")?;
    Ok(())
}

impl Storage for DirectoryStorageBackend {
    fn write(&mut self, match_replay: &MatchReplay) -> anyhow::Result<()> {
        let path = self.path.join(format!("{}.json", match_replay.match_id));
        info!(
            "Writing match replay to file: {}",
            path.clone().to_str().unwrap_or("Path not found")
        );
        let file = File::create(path)?;
        let mut writer = BufWriter::new(file);

        for match_item in match_replay {
            write_line(&mut writer, &match_item)?;
        }
        info!("Match replay written to file");
        Ok(())
    }
}
