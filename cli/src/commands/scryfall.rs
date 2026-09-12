use std::{collections::HashMap, time::Duration};

use anyhow::Context;
use reqwest::{StatusCode, header::RETRY_AFTER};
use tracing::warn;

/// User agent sent with all Scryfall requests.
pub const USER_AGENT: &str = "arenabuddy/1.0";

/// Number of attempts before giving up on a transiently failing request.
const MAX_ATTEMPTS: u32 = 8;
/// Backoff before the first retry; doubles on each subsequent retry.
const INITIAL_BACKOFF: Duration = Duration::from_secs(2);
/// Upper bound on the computed exponential backoff.
const MAX_BACKOFF: Duration = Duration::from_secs(60);
/// Upper bound honored for a server-supplied `Retry-After` header.
const MAX_RETRY_AFTER: Duration = Duration::from_secs(300);

/// Send a request, retrying on rate limits (429), server errors (5xx) and
/// transport failures with exponential backoff.
///
/// A `Retry-After` header on the response takes precedence over the computed
/// backoff. Any other response (including 4xx errors such as 404) is returned
/// as-is for the caller to interpret.
pub async fn send_with_retry(request: reqwest::RequestBuilder) -> anyhow::Result<reqwest::Response> {
    let mut backoff = INITIAL_BACKOFF;
    let mut attempt = 0;
    loop {
        attempt += 1;
        let req = request.try_clone().context("Scryfall request is not cloneable")?;
        let (reason, wait) = match req.send().await {
            Ok(response) if !is_transient(response.status()) => return Ok(response),
            Ok(response) => (format!("status {}", response.status()), retry_after(&response)),
            Err(err) if err.is_connect() || err.is_timeout() || err.is_request() => (err.to_string(), None),
            Err(err) => return Err(err.into()),
        };

        if attempt >= MAX_ATTEMPTS {
            anyhow::bail!("Scryfall request failed after {attempt} attempts: {reason}");
        }

        let wait = wait.unwrap_or(backoff);
        warn!(
            "Scryfall request failed ({}); retrying in {:.1}s (attempt {}/{})",
            reason,
            wait.as_secs_f64(),
            attempt,
            MAX_ATTEMPTS
        );
        tokio::time::sleep(wait).await;
        backoff = (backoff * 2).min(MAX_BACKOFF);
    }
}

/// Whether a status code indicates a condition that may clear on retry.
fn is_transient(status: StatusCode) -> bool {
    status == StatusCode::TOO_MANY_REQUESTS || status.is_server_error()
}

/// Parse a `Retry-After` header expressed in seconds, if present.
fn retry_after(response: &reqwest::Response) -> Option<Duration> {
    response
        .headers()
        .get(RETRY_AFTER)?
        .to_str()
        .ok()?
        .trim()
        .parse::<u64>()
        .ok()
        .map(|secs| Duration::from_secs(secs).min(MAX_RETRY_AFTER))
}

/// Fetch every card in a Scryfall set, indexing each page via `extract`.
///
/// Returns `Ok(None)` when the set search returns 404 (unknown set); otherwise
/// the accumulated map across all pages. `rate_limit` is the delay between page
/// requests.
pub async fn fetch_set<F>(
    client: &reqwest::Client,
    base_url: &str,
    set: &str,
    rate_limit: Duration,
    extract: F,
) -> anyhow::Result<Option<HashMap<String, serde_json::Value>>>
where
    F: Fn(&mut HashMap<String, serde_json::Value>, &serde_json::Value),
{
    let set_query = format!("e:{set}");
    let query = [
        ("include_variations", "true"),
        ("order", "set"),
        ("q", set_query.as_str()),
        ("unique", "cards"),
    ];

    let response = send_with_retry(client.get(format!("{base_url}/cards/search")).query(&query)).await?;

    if response.status() == StatusCode::NOT_FOUND {
        return Ok(None);
    }
    response.error_for_status_ref()?;

    let mut data: serde_json::Value = response.json().await?;
    let mut results = HashMap::new();
    extract(&mut results, &data);
    paginate(client, &mut data, &mut results, rate_limit, extract).await?;
    Ok(Some(results))
}

/// Paginate through Scryfall search results, calling `extract` on each page.
///
/// `data` is the JSON response from the initial request. This function follows
/// `next_page` links until there are no more pages, sleeping `rate_limit` between
/// requests to respect Scryfall's rate limit.
pub async fn paginate<F>(
    client: &reqwest::Client,
    data: &mut serde_json::Value,
    results: &mut HashMap<String, serde_json::Value>,
    rate_limit: Duration,
    extract: F,
) -> anyhow::Result<()>
where
    F: Fn(&mut HashMap<String, serde_json::Value>, &serde_json::Value),
{
    while let Some(next_page) = data["next_page"].as_str() {
        tokio::time::sleep(rate_limit).await;
        let response = send_with_retry(client.get(next_page)).await?;
        response.error_for_status_ref()?;
        *data = response.json().await?;
        extract(results, data);
    }
    Ok(())
}
