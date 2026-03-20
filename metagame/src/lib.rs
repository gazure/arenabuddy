pub mod classification;
pub mod scraper;

use std::path::PathBuf;

use arenabuddy_data::{ArenabuddyRepository, MatchDB, MetagameRepository, metagame_repository::MetagameStatsResult};
use clap::{Parser, Subcommand};
use tracing::info;

#[derive(Parser)]
#[command(name = "arenabuddy-metagame", about = "MTGGoldfish metagame scraper")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Scrape data from `MTGGoldfish`
    Scrape {
        /// Read pages from a local directory instead of fetching from the web.
        #[arg(long)]
        local_dir: Option<PathBuf>,

        #[command(subcommand)]
        target: ScrapeTarget,
    },
    /// Show database statistics
    Stats {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
}

#[derive(Subcommand)]
enum ScrapeTarget {
    /// Scrape tournament decklists via date range search
    Tournaments {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Start date (MM/DD/YYYY). Defaults to 14 days ago.
        #[arg(long)]
        from: Option<String>,

        /// End date (MM/DD/YYYY). Defaults to today.
        #[arg(long)]
        to: Option<String>,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
    /// Scrape tournaments by their `MTGGoldfish` IDs (e.g. 62212 62177)
    Tournament {
        /// One or more numeric `MTGGoldfish` tournament IDs
        #[arg(required = true)]
        ids: Vec<i32>,

        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
    /// Scrape metagame archetype index
    Metagame {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
}

/// # Panics
///
/// Panics if the tokio runtime cannot be created.
pub fn run() {
    let cli = Cli::parse();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let rt = tokio::runtime::Runtime::new().expect("failed to create tokio runtime");
    rt.block_on(async {
        if let Err(e) = run_command(cli.command).await {
            tracing::error!("Error: {e:#}");
            std::process::exit(1);
        }
    });
}

async fn connect_and_init(db_url: &str) -> anyhow::Result<MatchDB> {
    let cards = arenabuddy_core::cards::CardsDatabase::default();
    let db = MatchDB::new(Some(db_url), cards).await?;
    db.init().await?;
    Ok(db)
}

fn print_stats(stats: &MetagameStatsResult, format: &str) {
    info!("=== {format} metagame stats ===");
    info!("Tournaments: {}", stats.tournament_count);
    info!("Archetypes:  {}", stats.archetype_count);
    info!("Decks:       {}", stats.deck_count);
    info!("Card entries: {}", stats.card_count);
}

async fn run_command(command: Commands) -> anyhow::Result<()> {
    match command {
        Commands::Scrape { local_dir, target } => {
            let fetcher = match local_dir {
                Some(dir) => scraper::Fetcher::local(&dir)?,
                None => scraper::Fetcher::http(),
            };
            match target {
                ScrapeTarget::Tournaments { format, from, to, db } => {
                    let today = chrono::Utc::now().date_naive();
                    let from_date = match &from {
                        Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")?,
                        None => today - chrono::Days::new(14),
                    };
                    let to_date = match &to {
                        Some(s) => chrono::NaiveDate::parse_from_str(s, "%m/%d/%Y")?,
                        None if from.is_some() => from_date,
                        None => today,
                    };
                    let repo = connect_and_init(&db).await?;
                    scraper::tournament::scrape_tournaments(&repo, &fetcher, &format, from_date, to_date).await?;
                }
                ScrapeTarget::Tournament { ids, format, db } => {
                    let repo = connect_and_init(&db).await?;
                    for id in ids {
                        scraper::tournament::scrape_single_tournament(&repo, &fetcher, id, &format).await?;
                    }
                }
                ScrapeTarget::Metagame { format, db } => {
                    let repo = connect_and_init(&db).await?;
                    scraper::metagame::scrape_metagame(&repo, &fetcher, &format).await?;
                }
            }
        }
        Commands::Stats { format, db } => {
            let repo = connect_and_init(&db).await?;
            let stats = repo.metagame_stats(&format).await?;
            print_stats(&stats, &format);
        }
    }
    Ok(())
}
