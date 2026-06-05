use clap::Parser;

mod commands;
mod errors;

use commands::{Commands, DeckCommands};
pub use errors::{Error, ParseError, Result};

#[derive(Debug, Parser)]
#[command(about = "Tries to scrape useful data from mtga detailed logs")]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[arg(long, global = true)]
    debug: bool,
}

pub async fn run() -> Result<()> {
    let cli = Cli::parse();
    tracing_subscriber::fmt()
        .pretty()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
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
            output,
        } => {
            commands::scrape::execute(scryfall_host, seventeen_lands_host, output).await?;
        }

        Commands::ScrapeMtga {
            mtga_path,
            scryfall_host,
            output,
        } => {
            commands::scrape_mtga::execute(mtga_path.as_ref(), scryfall_host, output).await?;
        }

        Commands::Repl { cards_db } => {
            commands::repl::execute(cards_db)?;
        }

        Commands::Metagame { command } => {
            commands::metagame::execute(command).await?;
        }

        Commands::LoadCards { cards_db, db } => {
            commands::load_cards::execute(cards_db.as_ref(), db).await?;
        }

        Commands::EventLog {
            player_log,
            cards_db,
            output,
            game,
        } => {
            commands::event_log::execute(player_log, cards_db.as_ref(), output.as_ref(), *game).await?;
        }

        Commands::Deck { command } => match command {
            DeckCommands::Show {
                cards_db,
                db,
                match_id,
                game,
                clipboard,
                input,
                main,
                side,
            } => {
                commands::deck::show(commands::deck::DeckShowOpts {
                    cards_db: cards_db.as_path(),
                    db_url: db.as_deref(),
                    match_id: match_id.as_deref(),
                    game: *game,
                    input: input.as_ref().map(std::path::PathBuf::as_path),
                    clipboard: *clipboard,
                    main: main.as_deref(),
                    side: side.as_deref(),
                })
                .await?;
            }
        },
    }

    Ok(())
}
