use std::path::PathBuf;

use clap::Subcommand;

// Constants used in command definitions
pub const SCRYFALL_HOST_DEFAULT: &str = "https://api.scryfall.com";
pub const SEVENTEEN_LANDS_HOST_DEFAULT: &str = "https://17lands-public.s3.amazonaws.com";

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// Parse Arena log files to extract match data
    Parse {
        #[arg(short, long, help = "Location of Player.log file")]
        player_log: PathBuf,

        #[arg(short, long, help = "Directory to write replay output files")]
        output_dir: Option<PathBuf>,

        #[arg(short, long, help = "Database to write match data to")]
        db: Option<PathBuf>,

        #[arg(short, long, help = "Database of cards to reference")]
        cards_db: Option<PathBuf>,

        #[arg(long, action = clap::ArgAction::SetTrue, help = "Enable debug logging")]
        debug: bool,

        #[arg(
            short, long, action = clap::ArgAction::SetTrue,
            help = "Wait for new events on Player.log, useful if you are actively playing MTGA"
        )]
        follow: bool,
    },

    /// Scrape card data from online sources
    Scrape {
        #[arg(long, help = "Scryfall API base URL", default_value = SCRYFALL_HOST_DEFAULT)]
        scryfall_host: String,

        #[arg(long, help = "17Lands data base URL", default_value = SEVENTEEN_LANDS_HOST_DEFAULT)]
        seventeen_lands_host: String,

        #[arg(
            long,
            help = "Output directory for scraped data",
            default_value = "scrape_data"
        )]
        output_dir: PathBuf,
    },

    /// Start an interactive REPL for card searches, analytics, and file info
    Repl {
        #[arg(short, long, help = "Database of cards to reference")]
        cards_db: PathBuf,
    },
}
