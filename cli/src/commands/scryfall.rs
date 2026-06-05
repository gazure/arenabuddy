use std::{collections::HashMap, time::Duration};

use reqwest::StatusCode;

/// User agent sent with all Scryfall requests.
pub const USER_AGENT: &str = "arenabuddy/1.0";

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

    let response = client
        .get(format!("{base_url}/cards/search"))
        .query(&query)
        .send()
        .await?;

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
        let response = client.get(next_page).send().await?;
        response.error_for_status_ref()?;
        *data = response.json().await?;
        extract(results, data);
    }
    Ok(())
}
