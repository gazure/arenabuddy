use std::{
    io,
    path::{Path, PathBuf},
};

use same_file::Handle;
use tokio::{
    fs::File,
    io::{AsyncReadExt, AsyncSeekExt, SeekFrom},
};

const CHUNK_BYTES: usize = 64 * 1024;
const CHECKPOINT_BYTES: usize = 256;

pub(super) enum ReadResult {
    Bytes(Vec<u8>),
    Boundary,
    Eof,
}

#[derive(Debug)]
struct OpenLog {
    file: File,
    identity: Handle,
    offset: u64,
    prefix: Vec<u8>,
    tail: Vec<u8>,
    limit: Option<u64>,
}

impl OpenLog {
    async fn open(path: PathBuf, snapshot: bool) -> io::Result<Self> {
        let (file, identity, length) = tokio::task::spawn_blocking(move || {
            let file = std::fs::File::open(path)?;
            let metadata = file.metadata()?;
            if !metadata.is_file() {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "player log is not a regular file",
                ));
            }
            let identity = Handle::from_file(file.try_clone()?)?;
            Ok::<_, io::Error>((file, identity, metadata.len()))
        })
        .await
        .map_err(io::Error::other)??;
        Ok(Self {
            file: File::from_std(file),
            identity,
            offset: 0,
            prefix: Vec::new(),
            tail: Vec::new(),
            limit: snapshot.then_some(length),
        })
    }

    async fn unchanged_prefix(&self, candidate: &mut Self) -> io::Result<bool> {
        if candidate.file.metadata().await?.len() < self.offset {
            return Ok(false);
        }
        for (offset, expected) in [(0, &self.prefix), (self.offset - self.tail.len() as u64, &self.tail)] {
            if expected.is_empty() {
                continue;
            }
            candidate.file.seek(SeekFrom::Start(offset)).await?;
            let mut bytes = vec![0; expected.len()];
            match candidate.file.read_exact(&mut bytes).await {
                Ok(_) if bytes == *expected => {}
                Ok(_) => return Ok(false),
                Err(error) if error.kind() == io::ErrorKind::UnexpectedEof => return Ok(false),
                Err(error) => return Err(error),
            }
        }
        Ok(true)
    }

    async fn read(&mut self) -> io::Result<ReadResult> {
        let capacity = self.limit.map_or(CHUNK_BYTES, |limit| {
            usize::try_from(limit.saturating_sub(self.offset))
                .unwrap_or(usize::MAX)
                .min(CHUNK_BYTES)
        });
        if capacity == 0 {
            return Ok(ReadResult::Eof);
        }
        let mut bytes = vec![0; capacity];
        let count = self.file.read(&mut bytes).await?;
        if count == 0 {
            return Ok(ReadResult::Eof);
        }
        bytes.truncate(count);
        self.offset += count as u64;
        let prefix_count = (CHECKPOINT_BYTES - self.prefix.len()).min(count);
        self.prefix.extend_from_slice(&bytes[..prefix_count]);
        if count >= CHECKPOINT_BYTES {
            self.tail = bytes[count - CHECKPOINT_BYTES..].to_vec();
        } else {
            self.tail.extend_from_slice(&bytes);
            let excess = self.tail.len().saturating_sub(CHECKPOINT_BYTES);
            self.tail.drain(..excess);
        }
        Ok(ReadResult::Bytes(bytes))
    }
}

#[derive(Debug)]
pub(super) struct LogFollower {
    path: PathBuf,
    follow: bool,
    current: Option<OpenLog>,
}

impl LogFollower {
    pub async fn new(path: &Path, follow: bool) -> io::Result<Self> {
        let path = std::path::absolute(path)?;
        let current = match OpenLog::open(path.clone(), !follow).await {
            Ok(file) => Some(file),
            Err(error) if follow && error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Self { path, follow, current })
    }

    pub async fn read(&mut self) -> io::Result<ReadResult> {
        if !self.follow {
            return match &mut self.current {
                Some(current) => current.read().await,
                None => Ok(ReadResult::Eof),
            };
        }
        let mut candidate = match OpenLog::open(self.path.clone(), false).await {
            Ok(file) => Some(file),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        match (&mut self.current, &mut candidate) {
            (Some(current), Some(next)) if current.identity == next.identity => {
                if !current.unchanged_prefix(next).await? {
                    next.file.seek(SeekFrom::Start(0)).await?;
                    self.current = candidate;
                    return Ok(ReadResult::Boundary);
                }
                current.read().await
            }
            (Some(current), _) => {
                // A renamed file can still have unread bytes. Drain those
                // before switching to the replacement at the configured path.
                let result = current.read().await?;
                if matches!(result, ReadResult::Eof) && candidate.is_some() {
                    self.current = candidate;
                    Ok(ReadResult::Boundary)
                } else {
                    Ok(result)
                }
            }
            (None, _) => {
                self.current = candidate;
                match &mut self.current {
                    Some(current) => current.read().await,
                    None => Ok(ReadResult::Eof),
                }
            }
        }
    }
}
