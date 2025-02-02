#![deny(clippy::pedantic)]
use anyhow::Result;
use tracing::{error, info};

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    info!("scraping logs");
    match std::env::args().nth(1) {
        Some(target) => match target.as_str() {
            "scrape" => arenabuddy_scraper::scrape().await?,
            "process" => arenabuddy_scraper::process(None, None).await?,
            "clean" => arenabuddy_scraper::clean().await?,
            _ => {
                error!("Unknown target {}", target);
                std::process::exit(1);
            }
        },
        None => {
            error!("no option selected");
        }
    }

    Ok(())
}
