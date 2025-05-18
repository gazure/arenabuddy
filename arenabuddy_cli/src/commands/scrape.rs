use std::path::PathBuf;

use anyhow::Result;
use tracing::info;

/// Execute the Scrape command
pub async fn execute(
    scryfall_host: &str,
    seventeen_lands_host: &str,
    output_dir: &PathBuf,
) -> Result<()> {
    // Create output directory if it doesn't exist
    tokio::fs::create_dir_all(output_dir).await?;
    
    // Set up file paths
    let scryfall_output = output_dir.join("all_cards.json");
    let seventeen_lands_output = output_dir.join("seventeen_lands.csv");
    
    // Scrape data from both sources
    info!("Scraping Scryfall data...");
    scrape_scryfall(scryfall_host, &scryfall_output).await?;
    
    info!("Scraping 17Lands data...");
    scrape_seventeen_lands(seventeen_lands_host, &seventeen_lands_output).await?;
    
    info!("Scraping completed successfully");
    Ok(())
}

/// Scrape card data from Scryfall API
async fn scrape_scryfall(base_url: &str, output_file: &PathBuf) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;

    // Get bulk data endpoint
    let response = client.get(format!("{base_url}/bulk-data")).send().await?;

    info!("Response: {}", response.status());
    response.error_for_status_ref()?;

    let data: serde_json::Value = response.json().await?;

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

                let cards_data: serde_json::Value = cards_response.json().await?;

                // Create output directory if it doesn't exist
                if let Some(parent) = output_file.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Write to file using tokio
                let file = tokio::fs::File::create(output_file).await?;
                let mut writer = tokio::io::BufWriter::new(file);
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

/// Scrape card data from 17Lands
async fn scrape_seventeen_lands(base_url: &str, output_file: &PathBuf) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    let data = response.text().await?;

    // Create output directory if it doesn't exist
    if let Some(parent) = output_file.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Write to file using tokio
    let file = tokio::fs::File::create(output_file).await?;
    let mut writer = tokio::io::BufWriter::new(file);
    tokio::io::AsyncWriteExt::write_all(&mut writer, data.as_bytes()).await?;

    Ok(data)
}