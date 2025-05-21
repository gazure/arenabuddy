use std::process;

use anyhow::Result;
use clap::Parser;
use tokio::runtime::Runtime;

use crate::commands::Commands;

mod commands;

#[derive(Debug, Parser)]
#[command(about = "Tries to scrape useful data from mtga detailed logs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn main() {
    // Initialize tracing for logging
    tracing_subscriber::fmt().init();
    if let Err(e) = run() {
        eprintln!("Error: {e}");
        process::exit(1);
    }
}

fn run() -> Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Parse {
            player_log,
            output_dir,
            db,
            cards_db,
            debug,
            follow,
        } => {
            commands::parse::execute(player_log, output_dir, db, cards_db, *debug, *follow)?;
        }
        Commands::Scrape {
            scryfall_host,
            seventeen_lands_host,
            output_dir,
        } => {
            let rt = Runtime::new()?;
            rt.block_on(commands::scrape::execute(
                scryfall_host,
                seventeen_lands_host,
                output_dir,
            ))?;
        }
        Commands::Info { file } => {
            commands::info::execute(file)?;
        }
    }

    Ok(())
}
