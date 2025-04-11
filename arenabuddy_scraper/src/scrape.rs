use anyhow::Result;
use reqwest::Client;
use serde_json::Value;
use tokio::io::{AsyncWriteExt, BufWriter};
use tracing::info;

pub(crate) const SEVENTEEN_LANDS_OUT: &str = "scrape_data/seventeen_lands.csv";
pub(crate) const SCRYFALL_OUT: &str = "scrape_data/all_cards.json";

const SCRYFALL_HOST_DEFAULT: &str = "https://api.scryfall.com";
const SEVENTEEN_LANDS_HOST_DEFAULT: &str = "https://17lands-public.s3.amazonaws.com";

/// # Errors
///
/// Will error if underlying network or io fails
pub async fn scrape_scryfall(base_url: &str) -> Result<()> {
    let client = Client::builder().user_agent("cardscraper/1.0").build()?;

    // Get bulk data endpoint
    let response = client.get(format!("{base_url}/bulk-data")).send().await?;

    info!("Response: {}", response.status());
    response.error_for_status_ref()?;

    let data: Value = response.json().await?;

    // Find and download all_cards data
    let Some(bulk_data) = data.get("data").and_then(|d| d.as_array()) else {
        anyhow::bail!("Could not find all_cards data")
    };
    for item in bulk_data {
        if item["type"] == "all_cards" {
            if let Some(download_uri) = item["download_uri"].as_str() {
                info!("Downloading {}", download_uri);

                let cards_response = client.get(download_uri).send().await?;
                cards_response.error_for_status_ref()?;

                let cards_data: Value = cards_response.json().await?;

                // Write to file using tokio
                let file = tokio::fs::File::create(SCRYFALL_OUT).await?;
                let mut writer = BufWriter::new(file);
                tokio::io::AsyncWriteExt::write_all(
                    &mut writer,
                    serde_json::to_string(&cards_data)?.as_bytes(),
                )
                .await?;
                break;
            }
        }
    }
    Ok(())
}

/// # Errors
///
/// Will error if underlying network or io fails
pub async fn scrape_seventeen_lands(base_url: &str) -> Result<String> {
    let client = Client::builder().user_agent("cardscraper/1.0").build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    let data = response.text().await?;

    // Write to file using tokio
    let file = tokio::fs::File::create(SEVENTEEN_LANDS_OUT).await?;
    let mut writer = BufWriter::new(file);
    writer.write_all(data.as_bytes()).await?;

    Ok(data)
}

/// # Errors
///
/// Will error if underlying network/io fails
pub async fn scrape() -> Result<()> {
    info!("scraping scryfall");
    scrape_scryfall(SCRYFALL_HOST_DEFAULT).await?;
    info!("scraping 17lands");
    scrape_seventeen_lands(SEVENTEEN_LANDS_HOST_DEFAULT).await?;
    Ok(())
}
