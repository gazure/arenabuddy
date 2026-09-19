use std::path::Path;

use notify::{Event, EventKind, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::mpsc;

pub(super) struct LogWatcher {
    _watcher: RecommendedWatcher,
    pub wakeups: mpsc::Receiver<()>,
}

impl LogWatcher {
    pub fn new(path: &Path) -> crate::Result<Self> {
        let path = std::path::absolute(path)?;
        let parent = path.parent().unwrap_or(Path::new(".")).canonicalize()?;
        let target = parent.join(
            path.file_name()
                .ok_or_else(|| crate::Error::Io("missing log filename".into()))?,
        );
        let (tx, wakeups) = mpsc::channel(1);
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let relevant = match result {
                Ok(event) => {
                    matches!(
                        event.kind,
                        EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_)
                    ) && event.paths.iter().any(|path| path == &target)
                }
                // A backend error is a hint to poll, never proof of rotation.
                Err(_) => true,
            };
            if relevant {
                let _ = tx.try_send(());
            }
        })
        .map_err(|error| crate::Error::Io(error.to_string()))?;
        watcher
            .watch(&parent, RecursiveMode::NonRecursive)
            .map_err(|error| crate::Error::Io(error.to_string()))?;
        Ok(Self {
            _watcher: watcher,
            wakeups,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn dropping_watcher_releases_notification_sender() {
        let dir = tempfile::tempdir().expect("directory");
        let LogWatcher {
            _watcher: watcher,
            mut wakeups,
        } = LogWatcher::new(&dir.path().join("Player.log")).expect("watcher");
        drop(watcher);
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while wakeups.recv().await.is_some() {}
        })
        .await
        .expect("watcher released sender");
    }
}
