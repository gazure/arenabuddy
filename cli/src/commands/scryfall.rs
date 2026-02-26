use std::{collections::HashMap, time::Duration};

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
