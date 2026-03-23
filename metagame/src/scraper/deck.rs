use anyhow::Result;
use arenabuddy_data::metagame_models::{MetagameDeckCard, parse_deck_download};

use super::Fetcher;

/// Fetch and parse a deck's card list from the download endpoint.
pub async fn fetch_deck_cards(fetcher: &Fetcher, goldfish_deck_id: i32) -> Result<Vec<MetagameDeckCard>> {
    let text = fetcher.fetch(&format!("/deck/download/{goldfish_deck_id}")).await?;
    Ok(parse_deck_download(&text))
}
