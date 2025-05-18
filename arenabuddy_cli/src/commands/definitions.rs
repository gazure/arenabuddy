use std::path::PathBuf;
use clap::Subcommand;

// Constants used in command definitions
pub const SCRYFALL_HOST_DEFAULT: &str = "https://api.scryfall.com";
pub const SEVENTEEN_LANDS_HOST_DEFAULT: &str = "https://www.17lands.com";

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

    /// Process scraped card data into a usable format
    Process {
        #[arg(
            long,
            help = "Scryfall cards file to process",
            default_value = "scrape_data/all_cards.json"
        )]
        scryfall_cards_file: PathBuf,

        #[arg(
            long,
            help = "17Lands cards file to process",
            default_value = "scrape_data/seventeen_lands.csv"
        )]
        seventeen_lands_file: PathBuf,

        #[arg(
            long,
            help = "Output file for reduced Arena cards",
            default_value = "scrape_data/reduced_arena.pb"
        )]
        reduced_arena_out: PathBuf,

        #[arg(
            long,
            help = "Output file for merged card data",
            default_value = "src-tauri/data/cards-full.pb"
        )]
        merged_out: PathBuf,
    },

    /// Convert card data between formats
    Convert {
        #[command(subcommand)]
        action: ConvertAction,
    },
}

#[derive(Debug, Subcommand)]
pub enum ConvertAction {
    /// Convert JSON card data to Protocol Buffers format
    JsonToProto {
        /// Input JSON file path
        #[clap(short, long)]
        input: PathBuf,

        /// Output Protocol Buffers file path
        #[clap(short, long)]
        output: PathBuf,
    },

    /// Display information about a card data file
    Info {
        /// Protocol Buffers file path
        #[clap(short, long)]
        file: PathBuf,
    },
}
