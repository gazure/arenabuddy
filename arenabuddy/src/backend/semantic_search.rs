use arenabuddy_core::{
    cards::CardsDatabase,
    semantic_search::{self, SearchOptions, SearchResults},
};

use super::{BackgroundRuntime, credentials};

struct AbortOnDrop<T>(tokio::task::JoinHandle<T>);

impl<T> Drop for AbortOnDrop<T> {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// Searches cards on `runtime` using the key in OS storage.
///
/// Returns display-safe errors for missing credentials, failed searches, and
/// timeouts. Dropping this future cancels the background search.
pub(crate) async fn search(
    runtime: BackgroundRuntime,
    cards: CardsDatabase,
    options: SearchOptions,
) -> Result<SearchResults, String> {
    let mut task = AbortOnDrop(runtime.spawn(async move {
        let key = tokio::task::spawn_blocking(credentials::load_typesafe_key)
            .await
            .map_err(|_| "Could not access secure storage. Try again.".to_string())?
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "Save a TypeSafe API key in Settings to search with Jev.".to_string())?;
        tokio::time::timeout(
            std::time::Duration::from_secs(180),
            semantic_search::search(&cards, &options, &key),
        )
        .await
        .map_err(|_| "The search timed out. Try fewer candidates or a narrower query.".to_string())?
        .map_err(|error| error.to_string())
    }));
    (&mut task.0)
        .await
        .map_err(|_| "The search stopped unexpectedly. Try again.".to_string())?
}

#[cfg(test)]
mod tests {
    use super::AbortOnDrop;

    #[tokio::test]
    async fn dropping_search_cancels_background_work() {
        let (sender, receiver) = tokio::sync::oneshot::channel::<()>();
        let task = AbortOnDrop(tokio::spawn(async move {
            let _sender = sender;
            std::future::pending::<()>().await;
        }));
        drop(task);
        let result = tokio::time::timeout(std::time::Duration::from_secs(1), receiver).await;
        assert!(matches!(result, Ok(Err(_))));
    }
}
