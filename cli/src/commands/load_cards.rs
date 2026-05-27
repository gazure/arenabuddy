use std::path::PathBuf;

use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{ArenabuddyRepository, CardRepository, MatchDB};
use tracing::info;

use crate::Result;

pub async fn execute(cards_db_path: Option<&PathBuf>, db_url: &str) -> Result<()> {
    let cards_db = if let Some(path) = cards_db_path {
        info!("Loading cards from: {:?}", path);
        CardsDatabase::new(path)?
    } else {
        info!("Loading cards from embedded default database");
        CardsDatabase::default()
    };

    let card_count = cards_db.len();
    info!("Found {} cards to load", card_count);

    let db = MatchDB::new(Some(db_url), CardsDatabase::default()).await?;
    db.init().await?;

    let cards: Vec<_> = cards_db.values().cloned().collect();
    db.load_cards(&cards).await?;

    info!("Successfully loaded {} cards into PostgreSQL", card_count);
    Ok(())
}
