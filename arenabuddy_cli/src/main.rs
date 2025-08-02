use std::process;

use anyhow::Result;
use clap::Parser;
use tracing::Level;

use crate::commands::Commands;

mod commands;

#[derive(Debug, Parser)]
#[command(about = "Tries to scrape useful data from mtga detailed logs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    debug: bool,
}

#[tokio::main]
async fn main() {
    if let Err(e) = run().await {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

async fn run() -> Result<()> {
    let cli = Cli::parse();
    // Initialize logging
    tracing_subscriber::fmt()
        .with_max_level(if cli.debug { Level::DEBUG } else { Level::INFO })
        .init();

    match &cli.command {
        Commands::Parse {
            player_log,
            output_dir,
            db,
            cards_db,
            follow,
        } => {
            commands::parse::execute(
                player_log,
                output_dir.as_ref(),
                db.as_deref(),
                cards_db.as_ref(),
                *follow,
            )
            .await?;
        }
        Commands::Scrape {
            scryfall_host,
            seventeen_lands_host,
            output_dir,
        } => {
            commands::scrape::execute(scryfall_host, seventeen_lands_host, output_dir).await?;
        }

        Commands::Repl { cards_db } => {
            commands::repl::execute(cards_db)?;
        }
    }

    Ok(())
}
