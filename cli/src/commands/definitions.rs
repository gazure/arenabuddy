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

        #[arg(short, long, help = "Database url")]
        db: Option<String>,

        #[arg(short, long, help = "Database of cards to reference")]
        cards_db: Option<PathBuf>,

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

        #[arg(long, help = "Output directory for scraped data", default_value = "./cards.pb")]
        output: PathBuf,
    },

    /// Scrape card data from MTGA database and enrich with Scryfall
    ScrapeMtga {
        #[arg(long, help = "Path to MTGA installation directory")]
        mtga_path: Option<PathBuf>,

        #[arg(long, help = "Scryfall API base URL", default_value = SCRYFALL_HOST_DEFAULT)]
        scryfall_host: String,

        #[arg(long, help = "Output file for card database", default_value = "./cards.pb")]
        output: PathBuf,
    },

    /// Start an interactive REPL for card searches, analytics, and file info
    Repl {
        #[arg(short, long, help = "Database of cards to reference")]
        cards_db: PathBuf,
    },

    /// Metagame scraping and archetype classification
    Metagame {
        #[command(subcommand)]
        command: MetagameCommands,
    },

    /// Load card data into the database from a protobuf file or embedded defaults
    LoadCards {
        #[arg(short, long, help = "Path to cards protobuf file (uses embedded default if omitted)")]
        cards_db: Option<PathBuf>,

        #[arg(long, env = "ARENABUDDY_DATABASE_URL", help = "PostgreSQL database URL")]
        db: String,
    },

    /// Generate a structured event log from a Player.log file
    EventLog {
        #[arg(short, long, help = "Location of Player.log file")]
        player_log: PathBuf,

        #[arg(short, long, help = "Database of cards to reference")]
        cards_db: Option<PathBuf>,

        #[arg(short, long, help = "Output file (default: stdout)")]
        output: Option<PathBuf>,

        #[arg(long, help = "Filter to a specific game number")]
        game: Option<i32>,
    },

    /// Pretty-print decks from Postgres or JSON Arena card ID lists
    Deck {
        #[command(subcommand)]
        command: DeckCommands,
    },
}

#[derive(Debug, Subcommand)]
pub enum DeckCommands {
    /// Print a deck using `DeckDisplayRecord` names (Protobuf card DB)
    Show {
        #[arg(
            short = 'c',
            long,
            default_value = "data/cards-full.pb",
            help = "Protobuf card database (same as parse --cards-db)"
        )]
        cards_db: PathBuf,

        #[arg(
            long,
            env = "ARENABUDDY_DATABASE_URL",
            help = "PostgreSQL URL (required with --match-id)"
        )]
        db: Option<String>,

        #[arg(long, conflicts_with_all = ["clipboard", "main", "input"], help = "Match UUID (controller rows in `deck` table)")]
        match_id: Option<String>,

        #[arg(long, help = "Game number (`deck.game_number`); omit to print every game")]
        game: Option<i32>,

        #[arg(long, conflicts_with_all = ["match_id", "main", "input"], help = "Deck JSON from macOS clipboard (`pbpaste`)")]
        clipboard: bool,

        #[arg(value_name = "FILE", conflicts_with_all = ["match_id", "clipboard", "main"], help = "Deck JSON file, or `-` for stdin")]
        input: Option<PathBuf>,

        #[arg(long, conflicts_with_all = ["match_id", "clipboard", "input"], help = "Mainboard as JSON integer array")]
        main: Option<String>,

        #[arg(long, requires = "main", help = "Sideboard as JSON integer array")]
        side: Option<String>,
    },
}

#[derive(Debug, Subcommand)]
pub enum MetagameCommands {
    /// Scrape tournament decklists from `MTGGoldfish`
    ScrapeTournaments {
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

        /// Read pages from a local directory instead of fetching from the web
        #[arg(long)]
        local_dir: Option<PathBuf>,
    },

    /// Scrape metagame archetype index from `MTGGoldfish`
    ScrapeMetagame {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,

        /// Read pages from a local directory instead of fetching from the web
        #[arg(long)]
        local_dir: Option<PathBuf>,
    },

    /// Compute signature cards from scraped metagame data
    ComputeSignatures {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },

    /// Classify unclassified matches using signature cards
    Classify {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,

        /// Path to cards database file
        #[arg(short, long)]
        cards_db: Option<PathBuf>,
    },

    /// Show metagame database statistics
    Stats {
        /// MTG format (standard, pioneer, explorer, historic)
        #[arg(long, default_value = "standard")]
        format: String,

        /// Database URL
        #[arg(long, env = "DATABASE_URL")]
        db: String,
    },
}
