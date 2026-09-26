use std::path::PathBuf;

use arenabuddy_core::{
    cards::CardsDatabase,
    models::Card,
    semantic_search::{self, SearchOptions},
};
use clap::Args;

use crate::{Error, Result};

/// Configures a natural-language card search and its local filters.
#[derive(Debug, Args)]
pub struct SearchArgs {
    /// Description of the cards to find.
    pub query: String,
    /// Protobuf card database; defaults to the embedded database.
    #[arg(long)]
    pub cards_db: Option<PathBuf>,
    /// Maximum number of results to display.
    #[arg(long, default_value_t = 10, value_parser = clap::value_parser!(u16).range(1..=100))]
    pub limit: u16,
    /// Maximum number of local candidates to send to Jev.
    #[arg(long, default_value_t = 60, value_parser = clap::value_parser!(u16).range(1..=500))]
    pub candidates: u16,
    /// Require legality in this format according to the local database.
    #[arg(long, value_parser = ["standard", "alchemy", "historic", "timeless", "brawl", "standardbrawl", "gladiator", "pioneer"])]
    pub format: Option<String>,
    /// Maximum mana value, applied before ranking.
    #[arg(long, value_parser = clap::value_parser!(u16).range(0..=100))]
    pub max_mana_value: Option<u16>,
}

impl SearchArgs {
    /// Returns default search settings for a REPL query.
    pub fn for_query(query: String) -> Self {
        Self {
            query,
            cards_db: None,
            limit: 10,
            candidates: 60,
            format: None,
            max_mana_value: None,
        }
    }
}

/// Loads the selected card database and prints semantic search results.
///
/// Returns an error if the database, credentials, or `TypeSafe` request fails.
pub async fn execute(args: &SearchArgs) -> Result<()> {
    let db = args
        .cards_db
        .as_ref()
        .map(CardsDatabase::new)
        .transpose()?
        .unwrap_or_default();
    search_and_print(&db, args).await
}

/// Searches a loaded database and prints results from the shared Jev search.
pub async fn search_and_print(db: &CardsDatabase, args: &SearchArgs) -> Result<()> {
    let key = api_key()?;
    let options = SearchOptions {
        query: args.query.clone(),
        limit: args.limit,
        candidates: args.candidates,
        format: args.format.clone(),
        max_mana_value: args.max_mana_value,
    };
    eprintln!("Searching with Jev…");
    let result = semantic_search::search(db, &options, &key).await?;
    println!("Interpreted filters: {}", result.applied_filters);
    for notice in &result.notices {
        eprintln!("{notice}");
    }
    if result.ranked {
        println!(
            "Ranked {} candidates. Relevance: 0–3 (not a probability). Showing scores of at least 2.",
            result.ranked_count
        );
    } else {
        println!(
            "{} cards match the filters; showing up to {} in name order.",
            result.matching_count, args.limit
        );
    }
    for found in &result.matches {
        print_card(&found.card, found.score);
    }
    if result.matches.is_empty() {
        println!("No strong matches found. Try different wording or broader filters.");
    }
    eprintln!(
        "Usage: {} input tokens, {} output tokens.",
        result.usage.input_tokens, result.usage.output_tokens
    );
    Ok(())
}

fn print_card(card: &Card, score: Option<f64>) {
    let prefix = score.map_or_else(String::new, |score| format!("{score:.2}/3  "));
    println!("\n{prefix}{}  {}  (ID: {})", card.name, card.mana_cost, card.id);
    println!("  {} | Mana value: {} | Set: {}", card.type_line, card.cmc, card.set);
    if !card.oracle_text.is_empty() {
        println!("  {}", card.oracle_text.replace('\n', "\n  "));
    }
    for face in &card.card_faces {
        println!("  {}: {}", face.name, face.oracle_text.replace('\n', "\n  "));
    }
}

fn api_key() -> Result<String> {
    if let Ok(key) = std::env::var("TYPESAFE_API_KEY") {
        return nonempty_key(key);
    }
    // Read dotenv values without mutating the environment of the async runtime.
    let entries = dotenvy::dotenv_iter()
        .map_err(|_| Error::Config("Set TYPESAFE_API_KEY in your environment or a local .env file.".into()))?;
    for entry in entries {
        let (name, value) = entry.map_err(|_| Error::Config("Could not parse .env; check its syntax.".into()))?;
        if name == "TYPESAFE_API_KEY" {
            return nonempty_key(value);
        }
    }
    Err(Error::Config("TYPESAFE_API_KEY is missing from .env.".into()))
}

fn nonempty_key(key: String) -> Result<String> {
    if key.trim().is_empty() {
        Err(Error::Config("TYPESAFE_API_KEY is empty.".into()))
    } else {
        Ok(key)
    }
}
