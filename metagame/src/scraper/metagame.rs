use anyhow::Result;
use arenabuddy_data::MetagameRepository;
use scraper::{Html, Selector};
use tracing::info;

use super::Fetcher;

/// Scrape the metagame index page and upsert archetypes.
pub async fn scrape_metagame(repo: &impl MetagameRepository, fetcher: &Fetcher, format: &str) -> Result<()> {
    let html = fetcher.fetch(&format!("/metagame/{format}/full")).await?;
    let document = Html::parse_document(&html);
    let archetypes = parse_metagame_page(&document, fetcher.base_url());

    info!("Found {} archetypes for {format}", archetypes.len());

    for (name, url) in &archetypes {
        let id = repo.upsert_metagame_archetype(name, format, Some(url.as_str())).await?;
        info!("  Archetype: {name} (id={id})");
    }

    Ok(())
}

/// Parse archetype names and URLs from the metagame page HTML.
fn parse_metagame_page(document: &Html, base_url: &str) -> Vec<(String, String)> {
    let mut archetypes = Vec::new();

    let link_sel = Selector::parse("a[href*='/archetype/']").expect("valid selector");

    let mut seen = std::collections::HashSet::new();

    for link in document.select(&link_sel) {
        let Some(href) = link.value().attr("href") else {
            continue;
        };

        if !href.starts_with("/archetype/") {
            continue;
        }

        let name = link.text().collect::<String>().trim().to_string();
        if name.is_empty() || !seen.insert(name.clone()) {
            continue;
        }

        let full_url = format!("{base_url}{href}");
        archetypes.push((name, full_url));
    }

    archetypes
}
