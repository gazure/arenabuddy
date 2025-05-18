use std::{path::PathBuf, time::Duration};

use anyhow::{Context, Result};
use arenabuddy_core::{
    match_insights::MatchDB,
    processor::{EventSource, PlayerLogProcessor},
    proto_utils,
    replay::MatchReplayBuilder,
    storage_backends::{DirectoryStorageBackend, Storage},
};
use clap::{Parser, Subcommand};
use crossbeam::channel::{Receiver, select, unbounded};
use tokio::{io::AsyncWriteExt, runtime::Runtime};
use tracing::{error, info};

// Constants
const PLAYER_LOG_POLLING_INTERVAL: u64 = 1;
const SCRYFALL_HOST_DEFAULT: &str = "https://api.scryfall.com";
const SEVENTEEN_LANDS_HOST_DEFAULT: &str = "https://17lands-public.s3.amazonaws.com";

#[derive(Debug, Parser)]
#[command(name = "arenabuddy")]
#[command(about = "ArenabudCLI - A comprehensive tool for Magic: The Gathering Arena data management", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
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

    /// Clean up the scrape data directory
    Clean {
        #[arg(long, help = "Directory to clean", default_value = "scrape_data")]
        dir: PathBuf,
    },

    /// Convert card data between formats
    Convert {
        #[command(subcommand)]
        action: ConvertAction,
    },
}

#[derive(Debug, Subcommand)]
enum ConvertAction {
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

fn ctrl_c_channel() -> Result<Receiver<()>> {
    let (ctrl_c_tx, ctrl_c_rx) = unbounded();
    ctrlc::set_handler(move || {
        ctrl_c_tx.send(()).unwrap_or(());
    })?;
    Ok(ctrl_c_rx)
}

async fn scrape_scryfall(base_url: &str, output_file: &PathBuf) -> Result<()> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;

    // Get bulk data endpoint
    let response = client.get(format!("{base_url}/bulk-data")).send().await?;

    info!("Response: {}", response.status());
    response.error_for_status_ref()?;

    let data: serde_json::Value = response.json().await?;

    // Find and download all_cards data
    let Some(bulk_data) = data.get("data").and_then(|d| d.as_array()) else {
        anyhow::bail!("Could not find all_cards data")
    };
    for item in bulk_data {
        if item["type"] == "all_cards" {
            if let Some(download_uri) = item["download_uri"].as_str() {
                info!("Downloading {}", download_uri);

                let cards_response = client.get(download_uri).send().await?;
                cards_response.error_for_status_ref()?;

                let cards_data: serde_json::Value = cards_response.json().await?;

                // Create output directory if it doesn't exist
                if let Some(parent) = output_file.parent() {
                    tokio::fs::create_dir_all(parent).await?;
                }

                // Write to file using tokio
                let file = tokio::fs::File::create(output_file).await?;
                let mut writer = tokio::io::BufWriter::new(file);
                tokio::io::AsyncWriteExt::write_all(
                    &mut writer,
                    serde_json::to_string(&cards_data)?.as_bytes(),
                )
                .await?;
                break;
            }
        }
    }
    Ok(())
}

async fn scrape_seventeen_lands(base_url: &str, output_file: &PathBuf) -> Result<String> {
    let client = reqwest::Client::builder()
        .user_agent("arenabuddy/1.0")
        .build()?;
    let url = format!("{base_url}/analysis_data/cards/cards.csv");

    let response = client.get(&url).send().await?;
    info!("Response {}: {}", url, response.status());
    response.error_for_status_ref()?;

    let data = response.text().await?;

    // Create output directory if it doesn't exist
    if let Some(parent) = output_file.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    // Write to file using tokio
    let file = tokio::fs::File::create(output_file).await?;
    let mut writer = tokio::io::BufWriter::new(file);
    tokio::io::AsyncWriteExt::write_all(&mut writer, data.as_bytes()).await?;

    Ok(data)
}

async fn clean_directory(dir: &PathBuf) -> Result<()> {
    if !dir.exists() {
        info!(
            "Directory {} doesn't exist, nothing to clean",
            dir.display()
        );
        return Ok(());
    }

    let mut entries = tokio::fs::read_dir(dir).await?;
    let mut count = 0;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if path.is_file() {
            tokio::fs::remove_file(path).await?;
            count += 1;
        }
    }

    info!("Cleaned {} files from {}", count, dir.display());
    Ok(())
}

async fn process_cards(
    scryfall_cards_file: &PathBuf,
    seventeen_lands_file: &PathBuf,
    reduced_arena_out: &PathBuf,
    merged_out: &PathBuf,
) -> Result<()> {
    use std::collections::HashMap;

    use arenabuddy_core::proto::{Card, CardCollection, CardFace};
    use prost::Message;
    use serde_json::Value;

    info!("Processing cards from:");
    info!("  - Scryfall cards file: {}", scryfall_cards_file.display());
    info!("  - 17Lands file: {}", seventeen_lands_file.display());

    // Helper function to reduce arena cards
    fn reduce_arena_cards(card: &Value) -> Option<Card> {
        let id = card["arena_id"].as_i64()?;

        let mut proto_card = Card {
            id,
            set: card["set"].as_str()?.to_owned(),
            name: card["name"].as_str()?.to_owned(),
            lang: card["lang"].as_str()?.to_owned(),
            image_uri: card["image_uris"]["normal"]
                .as_str()
                .unwrap_or_default()
                .to_owned(),
            mana_cost: card["mana_cost"].as_str().unwrap_or_default().to_owned(),
            cmc: card["cmc"].as_f64().map_or(0, |f| f as i32),
            type_line: card["type_line"].as_str()?.to_owned(),
            layout: card["layout"].as_str()?.to_owned(),
            colors: Vec::new(),
            color_identity: Vec::new(),
            card_faces: Vec::new(),
        };

        // Parse array fields
        if let Some(colors) = card["colors"].as_array() {
            proto_card.colors = colors
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
        }

        if let Some(color_identity) = card["color_identity"].as_array() {
            proto_card.color_identity = color_identity
                .iter()
                .filter_map(|v| v.as_str().map(ToString::to_string))
                .collect();
        }

        // Parse card faces if present
        if let Some(faces) = card["card_faces"].as_array() {
            proto_card.card_faces = faces
                .iter()
                .filter_map(|face| {
                    if !face.is_object() {
                        return None;
                    }

                    let mut card_face = CardFace {
                        name: face["name"].as_str()?.to_string(),
                        type_line: face["type_line"].as_str()?.to_string(),
                        mana_cost: face["mana_cost"].as_str().unwrap_or_default().to_string(),
                        image_uri: None,
                        colors: Vec::new(),
                    };

                    // Optional fields
                    if let Some(image) = face
                        .get("image_uris")
                        .and_then(|uris| uris.get("normal"))
                        .and_then(|uri| uri.as_str())
                    {
                        card_face.image_uri = Some(image.to_string());
                    }

                    if let Some(face_colors) = face["colors"].as_array() {
                        card_face.colors = face_colors
                            .iter()
                            .filter_map(|c| c.as_str().map(ToString::to_string))
                            .collect();
                    }

                    Some(card_face)
                })
                .collect();
        }

        Some(proto_card)
    }

    fn extract_arena_id_cards(cards: &[Value]) -> Vec<Value> {
        cards
            .iter()
            .filter(|card| card["arena_id"].is_number())
            .cloned()
            .collect()
    }

    // Function to merge data from 17Lands with scryfall data
    fn merge(
        seventeen_lands_cards: &[HashMap<String, String>],
        cards_by_name: &HashMap<String, Card>,
        cards_by_id: &mut HashMap<i64, Card>,
        _arena_cards: &[Card],
    ) {
        // Create map of two-faced cards
        let card_names_with_2_faces: HashMap<String, String> = seventeen_lands_cards
            .iter()
            .filter_map(|card| {
                let name = card.get("name")?;
                if name.contains("//") {
                    Some((name.split("//").next()?.trim().to_string(), name.clone()))
                } else {
                    None
                }
            })
            .collect();

        for card in seventeen_lands_cards {
            if let (Some(card_name), Some(card_id_str)) = (card.get("name"), card.get("id")) {
                let card_name = card_names_with_2_faces
                    .get(card_name.split("//").next().unwrap_or("").trim())
                    .unwrap_or(card_name);

                if let (Ok(card_id), Some(card_by_name)) = (
                    card_id_str
                        .parse::<i64>()
                        .with_context(|| format!("Failed to parse card ID: {card_id_str}")),
                    cards_by_name.get(card_name),
                ) {
                    if card_id != 0 && !cards_by_id.contains_key(&card_id) {
                        info!("Adding card {} with ID {}", card_name, card_id);
                        let mut new_card = card_by_name.clone();
                        new_card.id = card_id;
                        cards_by_id.insert(card_id, new_card);
                    }
                }
            }
        }
    }

    // Load Scryfall cards
    info!(
        "Loading Scryfall cards from {}",
        scryfall_cards_file.display()
    );
    let mut contents = tokio::fs::read_to_string(scryfall_cards_file)
        .await
        .with_context(|| format!("Failed to read file: {}", scryfall_cards_file.display()))?;
    contents = contents.replace("\\u2014", "-");
    let scryfall_cards: Vec<Value> =
        serde_json::from_str(&contents).with_context(|| "Failed to parse JSON")?;

    info!("Loaded {} Scryfall cards", scryfall_cards.len());

    // Extract Arena cards and reduce them
    let arena_cards = extract_arena_id_cards(&scryfall_cards);
    let reduced_arena_cards: Vec<Card> =
        arena_cards.iter().filter_map(reduce_arena_cards).collect();

    let mut cards_by_id: HashMap<i64, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.id, card.clone()))
        .collect();

    let cards_by_name: HashMap<String, Card> = reduced_arena_cards
        .iter()
        .map(|card| (card.name.clone(), card.clone()))
        .collect();

    info!("Found {} Arena ID cards", reduced_arena_cards.len());

    // Write reduced arena cards to file as protocol buffer
    let mut card_collection = CardCollection::new();
    let reduced_arena_cards_clone = reduced_arena_cards.clone(); // Clone for later use
    for card in reduced_arena_cards {
        card_collection.cards.push(card);
    }

    let mut buf = Vec::new();
    card_collection
        .encode(&mut buf)
        .context("Failed to serialize reduced arena cards to protobuf")?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = reduced_arena_out.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let mut file = tokio::fs::File::create(reduced_arena_out)
        .await
        .with_context(|| format!("Failed to create file: {}", reduced_arena_out.display()))?;
    file.write_all(&buf)
        .await
        .with_context(|| format!("Failed to write to file: {}", reduced_arena_out.display()))?;

    info!(
        "Wrote reduced arena cards to {}",
        reduced_arena_out.display()
    );

    // Process 17Lands data
    info!(
        "Processing 17Lands data from {}",
        seventeen_lands_file.display()
    );
    let mut reader = csv::Reader::from_path(seventeen_lands_file).with_context(|| {
        format!(
            "Failed to open CSV file: {}",
            seventeen_lands_file.display()
        )
    })?;
    let seventeen_lands_cards: Vec<HashMap<String, String>> = reader
        .deserialize()
        .collect::<std::result::Result<_, _>>()
        .with_context(|| "Failed to parse CSV records")?;

    info!("Found {} 17Lands cards", seventeen_lands_cards.len());

    // Merge data
    merge(
        &seventeen_lands_cards,
        &cards_by_name,
        &mut cards_by_id,
        &reduced_arena_cards_clone,
    );

    // Write merged cards to file as protocol buffer
    let mut merged_collection = CardCollection::new();
    for (_, card) in cards_by_id {
        merged_collection.cards.push(card);
    }

    let mut buf = Vec::new();
    merged_collection
        .encode(&mut buf)
        .context("Failed to serialize merged cards to protobuf")?;

    // Create parent directory if it doesn't exist
    if let Some(parent) = merged_out.parent() {
        tokio::fs::create_dir_all(parent)
            .await
            .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
    }

    let mut file = tokio::fs::File::create(merged_out)
        .await
        .with_context(|| format!("Failed to create file: {}", merged_out.display()))?;
    file.write_all(&buf)
        .await
        .with_context(|| format!("Failed to write to file: {}", merged_out.display()))?;

    info!("Wrote merged cards to {}", merged_out.display());
    info!("Processing completed successfully!");

    Ok(())
}

fn run_parser(args: &Commands) -> Result<()> {
    if let Commands::Parse {
        player_log,
        output_dir,
        db,
        cards_db,
        debug,
        follow,
    } = args
    {
        tracing_subscriber::fmt()
            .with_max_level(if *debug {
                tracing::Level::DEBUG
            } else {
                tracing::Level::INFO
            })
            .init();

        let mut processor = PlayerLogProcessor::try_new(player_log.clone())?;
        let mut match_replay_builder = MatchReplayBuilder::new();
        let mut storage_backends: Vec<Box<dyn Storage>> = Vec::new();
        let cards_db = arenabuddy_core::cards::CardsDatabase::new(
            cards_db
                .clone()
                .unwrap_or_else(|| PathBuf::from("data/cards-full.pb")),
        )?;

        let ctrl_c_rx = ctrl_c_channel()?;
        if let Some(output_dir) = output_dir {
            std::fs::create_dir_all(output_dir)?;
            storage_backends.push(Box::new(DirectoryStorageBackend::new(output_dir.clone())));
        }

        if let Some(db_path) = db {
            let conn = rusqlite::Connection::open(db_path)?;
            let mut db = MatchDB::new(conn, cards_db);
            db.init()?;
            storage_backends.push(Box::new(db));
        }

        loop {
            select! {
                recv(ctrl_c_rx) -> _ => {
                    break;
                }
                default(Duration::from_secs(PLAYER_LOG_POLLING_INTERVAL)) => {
                    while let Ok(parse_output) = processor.get_next_event() {
                        if match_replay_builder.ingest_event(parse_output) {
                            match match_replay_builder.build() {
                                Ok(match_replay) => {
                                    for backend in &mut storage_backends {
                                        if let Err(e) = backend.write(&match_replay) {
                                            error!("Error writing replay to backend: {e}");
                                        }
                                    }
                                },
                                Err(err) => {
                                    error!("Error building match replay: {err}");
                                }
                            }
                            match_replay_builder = MatchReplayBuilder::new();
                        }
                    }
                    if !follow {
                        break;
                    }
                }
            }
        }
    }
    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Set up logging
    tracing_subscriber::fmt::init();

    match &cli.command {
        Commands::Parse { .. } => {
            run_parser(&cli.command)?;
        }

        Commands::Scrape {
            scryfall_host,
            seventeen_lands_host,
            output_dir,
        } => {
            let rt = Runtime::new()?;
            rt.block_on(async {
                let scryfall_file = output_dir.join("all_cards.json");
                let seventeen_lands_file = output_dir.join("seventeen_lands.csv");

                info!("Scraping card data...");
                info!("Scraping Scryfall data to {}", scryfall_file.display());
                scrape_scryfall(scryfall_host, &scryfall_file).await?;

                info!(
                    "Scraping 17Lands data to {}",
                    seventeen_lands_file.display()
                );
                scrape_seventeen_lands(seventeen_lands_host, &seventeen_lands_file).await?;

                info!("Scraping completed successfully!");
                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Process {
            scryfall_cards_file,
            seventeen_lands_file,
            reduced_arena_out,
            merged_out,
        } => {
            let rt = Runtime::new()?;
            rt.block_on(async {
                info!("Processing card data...");
                process_cards(
                    scryfall_cards_file,
                    seventeen_lands_file,
                    reduced_arena_out,
                    merged_out,
                )
                .await?;
                info!("Processing completed successfully!");
                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Clean { dir } => {
            let rt = Runtime::new()?;
            rt.block_on(async {
                info!("Cleaning directory: {}", dir.display());
                clean_directory(dir).await?;
                info!("Cleaning completed successfully!");
                Ok::<(), anyhow::Error>(())
            })?;
        }

        Commands::Convert { action } => {
            match action {
                ConvertAction::JsonToProto { input, output } => {
                    info!(
                        "Converting JSON to Protocol Buffers: {} -> {}",
                        input.display(),
                        output.display()
                    );

                    proto_utils::convert_json_to_proto_file(input, output).with_context(|| {
                        format!(
                            "Failed to convert {} to {}",
                            input.display(),
                            output.display()
                        )
                    })?;

                    info!("Conversion completed successfully!");
                }

                ConvertAction::Info { file } => {
                    info!("Displaying information about: {}", file.display());

                    let cards = proto_utils::load_card_collection_from_file(file)
                        .with_context(|| format!("Failed to load cards from {}", file.display()))?;

                    println!("Card collection: {}", file.display());
                    println!("Total cards: {}", cards.len());

                    // Display some statistics about the cards
                    let mut layouts = std::collections::HashMap::new();
                    let mut sets = std::collections::HashMap::new();

                    for card in &cards {
                        *layouts.entry(card.layout.clone()).or_insert(0) += 1;
                        *sets.entry(card.set.clone()).or_insert(0) += 1;
                    }

                    println!("\nLayouts:");
                    for (layout, count) in layouts {
                        println!("  {}: {}", layout, count);
                    }

                    println!("\nSets:");
                    for (set, count) in sets {
                        println!("  {}: {}", set, count);
                    }

                    // Print first 5 cards as a sample
                    if !cards.is_empty() {
                        println!("\nSample cards:");
                        for (i, card) in cards.iter().take(5).enumerate() {
                            println!("  {}. {} ({})", i + 1, card.name, card.set);
                        }
                    }
                }
            }
        }
    }

    Ok(())
}
