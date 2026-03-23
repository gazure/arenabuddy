use std::path::Path;

use anyhow::Context;
use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{ArenabuddyRepository, MatchDB, MetagameRepository};
use tracingx::info;

use super::definitions::MetagameCommands;
use crate::Result;

async fn connect(db_url: &str, cards: CardsDatabase) -> Result<MatchDB> {
    let db = MatchDB::new(Some(db_url), cards).await?;
    db.init().await?;
    Ok(db)
}

fn load_cards(cards_db: Option<&Path>) -> CardsDatabase {
    cards_db
        .and_then(|path| CardsDatabase::new(path).ok())
        .unwrap_or_default()
}

pub async fn execute(command: &MetagameCommands) -> Result<()> {
    match command {
        MetagameCommands::ScrapeTournaments {
            format,
            from,
            to,
            db,
            local_dir,
        } => {
            let fetcher = match local_dir {
                Some(dir) => arenabuddy_metagame::scraper::Fetcher::local(dir)?,
                None => arenabuddy_metagame::scraper::Fetcher::http(),
            };

            let today = chrono::Utc::now().date_naive();
            let from_date = match from {
                Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")
                    .context("invalid date format, expected MM/DD/YYYY")?,
                None => today - chrono::Days::new(14),
            };
            let to_date = match to {
                Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")
                    .context("invalid date format, expected MM/DD/YYYY")?,
                None if from.is_some() => from_date,
                None => today,
            };

            let repo = connect(db, CardsDatabase::default()).await?;
            arenabuddy_metagame::scraper::tournament::scrape_tournaments(&repo, &fetcher, format, from_date, to_date)
                .await?;
        }

        MetagameCommands::ScrapeMetagame { format, db, local_dir } => {
            let fetcher = match local_dir {
                Some(dir) => arenabuddy_metagame::scraper::Fetcher::local(dir)?,
                None => arenabuddy_metagame::scraper::Fetcher::http(),
            };

            let repo = connect(db, CardsDatabase::default()).await?;
            arenabuddy_metagame::scraper::metagame::scrape_metagame(&repo, &fetcher, format).await?;
        }

        MetagameCommands::ComputeSignatures { format, db } => {
            let repo = connect(db, CardsDatabase::default()).await?;
            let count = arenabuddy_metagame::classification::compute_signature_cards(&repo, format).await?;
            info!("Computed {count} signature cards for {format}");
        }

        MetagameCommands::Classify { format, db, cards_db } => {
            let cards = load_cards(cards_db.as_deref());
            let repo = connect(db, cards).await?;
            let count = arenabuddy_metagame::classification::classify_matches(&repo, format).await?;
            info!("Classified {count} matches for {format}");
        }

        MetagameCommands::Stats { format, db } => {
            let repo = connect(db, CardsDatabase::default()).await?;
            let stats = repo.metagame_stats(format).await?;
            info!("=== {format} metagame stats ===");
            info!("Tournaments: {}", stats.tournament_count);
            info!("Archetypes:  {}", stats.archetype_count);
            info!("Decks:       {}", stats.deck_count);
            info!("Card entries: {}", stats.card_count);
        }
    }

    Ok(())
}
