use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use reqwest::Client;
use tracing::debug;

const USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/131.0.0.0 Safari/537.36";
const BASE_URL: &str = "https://www.mtggoldfish.com";
const RATE_LIMIT_MS: u64 = 500;

/// Abstraction over fetching page content — either from HTTP or local files.
pub enum Fetcher {
    Http { client: Client, base_url: String },
    Local { dir: PathBuf },
}

impl Fetcher {
    /// Creates a new HTTP fetcher with default settings.
    ///
    /// # Panics
    ///
    /// Panics if the HTTP client cannot be built (e.g., due to invalid TLS configuration).
    pub fn http() -> Self {
        let client = Client::builder()
            .user_agent(USER_AGENT)
            .build()
            .expect("invalid http client");
        Self::Http {
            client,
            base_url: BASE_URL.to_string(),
        }
    }

    pub fn local(dir: &Path) -> Result<Self> {
        anyhow::ensure!(dir.is_dir(), "{} is not a directory", dir.display());
        Ok(Self::Local { dir: dir.to_path_buf() })
    }

    pub fn base_url(&self) -> &str {
        match self {
            Self::Http { base_url, .. } => base_url,
            Self::Local { .. } => BASE_URL,
        }
    }

    /// Fetch content at the given path (e.g. `/tournaments/standard?page=1`).
    pub async fn fetch(&self, path: &str) -> Result<String> {
        match self {
            Self::Http { client, base_url } => {
                tokio::time::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS)).await;
                let url = format!("{base_url}{path}");
                let resp = client
                    .get(&url)
                    .send()
                    .await?
                    .error_for_status()
                    .with_context(|| format!("request failed: {url}"))?;
                let final_url = resp.url().to_string();
                let text = resp
                    .text()
                    .await
                    .with_context(|| format!("failed to read body: {url}"))?;
                debug!("Fetched {url} -> {final_url} ({} bytes)", text.len());
                Ok(text)
            }
            Self::Local { dir } => {
                let filename = path_to_filename(path);
                let file_path = dir.join(&filename);
                std::fs::read_to_string(&file_path)
                    .with_context(|| format!("failed to read local file: {}", file_path.display()))
            }
        }
    }

    /// Fetch content, returning `None` on 400 Bad Request (used for pagination).
    pub async fn fetch_optional(&self, path: &str) -> Result<Option<String>> {
        match self {
            Self::Http { client, base_url } => {
                tokio::time::sleep(std::time::Duration::from_millis(RATE_LIMIT_MS)).await;
                let url = format!("{base_url}{path}");
                let resp = client
                    .get(&url)
                    .send()
                    .await
                    .with_context(|| format!("request failed: {url}"))?;
                if resp.status() == reqwest::StatusCode::BAD_REQUEST {
                    return Ok(None);
                }
                let text = resp
                    .error_for_status()
                    .with_context(|| format!("request failed: {url}"))?
                    .text()
                    .await
                    .with_context(|| format!("failed to read body: {url}"))?;
                Ok(Some(text))
            }
            Self::Local { dir } => {
                let filename = path_to_filename(path);
                let file_path = dir.join(&filename);
                match std::fs::read_to_string(&file_path) {
                    Ok(content) => Ok(Some(content)),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
                    Err(e) => Err(e).with_context(|| format!("failed to read local file: {}", file_path.display())),
                }
            }
        }
    }
}

/// Convert a URL path to a flat filename for local testing.
///
/// Examples:
/// - `/tournament_searches/create?...&page=1` -> `tournament_searches_create_..._page_1`
/// - `/tournament/62266` -> `tournament_62266`
/// - `/deck/download/7677856` -> `deck_download_7677856`
/// - `/metagame/standard/full` -> `metagame_standard_full`
fn path_to_filename(path: &str) -> String {
    path.trim_start_matches('/')
        .replace(['/', '?', '=', '&', '%', '[', ']', '+'], "_")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_to_filename() {
        assert_eq!(path_to_filename("/tournament/62266"), "tournament_62266");
        assert_eq!(path_to_filename("/deck/download/7677856"), "deck_download_7677856");
        assert_eq!(path_to_filename("/metagame/standard/full"), "metagame_standard_full");
        assert_eq!(
            path_to_filename("/tournament_searches/create?foo=bar&page=1"),
            "tournament_searches_create_foo_bar_page_1"
        );
    }
}
