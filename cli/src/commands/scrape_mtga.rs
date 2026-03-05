#![expect(clippy::too_many_lines)]
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    time::Duration,
};

use arenabuddy_core::models::{Card, CardCollection};
use reqwest::StatusCode;
use rusqlite::Connection;
use tracingx::{debug, info, warn};

use crate::{Error, Result};

const USER_AGENT: &str = "arenabuddy/1.0";
const SCRYFALL_RATE_LIMIT_MS: u64 = 150;

// Canonical Arena IDs for basic lands (used as fallback when not found in Scryfall)
const BASIC_LAND_FALLBACK_IDS: &[(&str, i64)] = &[
    ("Plains", 7193),
    ("Island", 7065),
    ("Swamp", 7347),
    ("Mountain", 7153),
    ("Forest", 6993),
    ("Snow-Covered Plains", 7193),
    ("Snow-Covered Island", 7065),
    ("Snow-Covered Swamp", 7347),
    ("Snow-Covered Mountain", 7153),
    ("Snow-Covered Forest", 6993),
];

/// Represents a card from MTGA database
#[derive(Debug)]
struct MtgaCard {
    grp_id: i64,
    expansion_code: String,
    collector_number: String,
    name: String,
}

/// Execute the `ScrapeMtga` command
pub async fn execute(mtga_path: Option<&PathBuf>, scryfall_host: &str, output: &Path) -> Result<()> {
    info!("Starting MTGA database scrape...");

    // 1. Find MTGA database
    let db_path = find_mtga_database(mtga_path)?;
    info!("Found MTGA database at: {}", db_path.display());

    // 2. Extract cards from MTGA database
    let mtga_cards = extract_mtga_cards(&db_path)?;
    info!("Extracted {} cards from MTGA database", mtga_cards.len());

    // 3. Enrich with Scryfall data
    let cards = enrich_with_scryfall(mtga_cards, scryfall_host).await?;
    info!("Successfully enriched {} cards with Scryfall data", cards.len());

    // 4. Save to protobuf
    let collection = CardCollection::with_cards(cards);
    save_card_collection(collection, output).await?;
    info!("Saved card collection to: {}", output.display());

    Ok(())
}

/// Find the MTGA database file
fn find_mtga_database(mtga_path: Option<&PathBuf>) -> Result<PathBuf> {
    let search_dir = if let Some(path) = mtga_path {
        path.clone()
    } else {
        // Get home directory using std
        let home_dir =
            dirs::home_dir().ok_or_else(|| Error::Config("Could not determine home directory".to_string()))?;

        // Default paths by platform
        #[cfg(target_os = "macos")]
        let base = home_dir.join("Library/Application Support/Steam/steamapps/common/MTGA/MTGA_Data/Downloads/Raw");

        #[cfg(target_os = "windows")]
        let base = {
            // On Windows, prefer LOCALAPPDATA for the standalone client
            let local_app_data = std::env::var("LOCALAPPDATA")
                .map(PathBuf::from)
                .unwrap_or_else(|_| home_dir.join("AppData/Local"));
            local_app_data.join("Programs/Wizards of the Coast/MTGA/MTGA_Data/Downloads/Raw")
        };

        #[cfg(target_os = "linux")]
        let base = home_dir.join(".steam/steam/steamapps/common/MTGA/MTGA_Data/Downloads/Raw");

        #[cfg(not(any(target_os = "macos", target_os = "windows", target_os = "linux")))]
        let base = {
            return Err(Error::Config(
                "Unsupported platform. Please specify --mtga-path manually.".to_string(),
            ));
        };

        base
    };

    if !search_dir.exists() {
        return Err(Error::MtgaDatabaseNotFound(search_dir.display().to_string()));
    }

    // Find Raw_CardDatabase_*.mtga file
    let entries = std::fs::read_dir(&search_dir)?;
    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if let Some(name) = path.file_name().and_then(|n| n.to_str())
            && name.starts_with("Raw_CardDatabase_")
            && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("mtga"))
        {
            return Ok(path);
        }
    }

    Err(Error::MtgaDatabaseNotFound(format!(
        "No Raw_CardDatabase_*.mtga file found in {}",
        search_dir.display()
    )))
}

/// Extract cards from MTGA `SQLite` database
fn extract_mtga_cards(db_path: &Path) -> Result<Vec<MtgaCard>> {
    let conn = Connection::open(db_path)?;

    let query = r"
        SELECT
            c.GrpId,
            c.ExpansionCode,
            c.CollectorNumber,
            l.Loc as name
        FROM Cards c
        JOIN Localizations_enUS l ON c.TitleId = l.LocId
        WHERE c.IsPrimaryCard = 1
          AND c.IsToken = 0
          AND l.Formatted = (
              SELECT MIN(Formatted)
              FROM Localizations_enUS
              WHERE LocId = c.TitleId
          )
        ORDER BY c.GrpId
    ";

    let mut stmt = conn.prepare(query)?;
    let cards = stmt
        .query_map([], |row| {
            Ok(MtgaCard {
                grp_id: row.get(0)?,
                expansion_code: row.get(1)?,
                collector_number: row.get(2)?,
                name: row.get(3)?,
            })
        })?
        .collect::<Result<Vec<_>, _>>()?;

    Ok(cards)
}

/// Get the canonical Arena ID for a basic land name, if applicable
fn get_basic_land_fallback_id(card_name: &str) -> Option<i64> {
    BASIC_LAND_FALLBACK_IDS
        .iter()
        .find(|(name, _)| *name == card_name)
        .map(|(_, id)| *id)
}

/// Enrich MTGA cards with Scryfall metadata using batch-by-set approach
async fn enrich_with_scryfall(mtga_cards: Vec<MtgaCard>, scryfall_host: &str) -> Result<Vec<Card>> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

    // Group MTGA cards by expansion code
    let mut cards_by_set: HashMap<String, Vec<MtgaCard>> = HashMap::new();
    for mtga_card in mtga_cards {
        cards_by_set
            .entry(mtga_card.expansion_code.clone())
            .or_default()
            .push(mtga_card);
    }

    let total_sets = cards_by_set.len();
    info!("Grouped cards into {} unique sets", total_sets);

    let mut cards = Vec::new();
    let mut cards_by_id = HashSet::new();
    let mut failed_cards = Vec::new();
    let mut processed_sets = 0;

    // Cache arena_id lookups to avoid repeated Scryfall fetches
    let mut arena_id_cache: HashMap<i64, serde_json::Value> = HashMap::new();

    // Process each set
    for (set_code, mtga_set_cards) in cards_by_set {
        processed_sets += 1;
        info!(
            "Processing set {}/{}: {} ({} cards)",
            processed_sets,
            total_sets,
            set_code,
            mtga_set_cards.len()
        );

        // Fetch all cards from this set from Scryfall
        let Some(scryfall_cards) = fetch_scryfall_set(&client, scryfall_host, &set_code).await? else {
            warn!(
                "Set '{}' not found in Scryfall, skipping {} cards",
                set_code,
                mtga_set_cards.len()
            );
            failed_cards.extend(mtga_set_cards);
            continue;
        };

        debug!(
            "Fetched {} cards from Scryfall for set {}",
            scryfall_cards.len(),
            set_code
        );

        // Match MTGA cards with Scryfall cards by collector number
        for mtga_card in mtga_set_cards {
            if cards_by_id.contains(&mtga_card.grp_id) {
                warn!(
                    "Duplicate arena_id {} for card '{}', skipping",
                    mtga_card.grp_id, mtga_card.name
                );
                continue;
            }

            // Look up by collector number in the Scryfall set data
            if let Some(scryfall_json) = scryfall_cards.get(&mtga_card.collector_number) {
                let mut card = Card::from_json(scryfall_json);
                card.id = mtga_card.grp_id; // Override with MTGA's arena ID

                // Verify consistency
                if let Some(scryfall_arena_id) = scryfall_json["arena_id"].as_i64()
                    && scryfall_arena_id != mtga_card.grp_id
                {
                    warn!(
                        "Arena ID mismatch for '{}': MTGA={}, Scryfall={}",
                        mtga_card.name, mtga_card.grp_id, scryfall_arena_id
                    );
                }

                cards.push(card);
                cards_by_id.insert(mtga_card.grp_id);
            } else {
                // Collector number miss — try fetching by the card's actual arena ID
                let card_json =
                    fetch_or_cache_by_arena_id(&client, scryfall_host, &mut arena_id_cache, mtga_card.grp_id).await?;

                // If that failed and it's a basic land, try the canonical fallback ID
                let card_json = match (card_json, get_basic_land_fallback_id(&mtga_card.name)) {
                    (Some(json), _) => Some(json),
                    (None, Some(fallback_id)) => {
                        debug!(
                            "Actual arena ID {} not found for '{}', trying fallback ID {}",
                            mtga_card.grp_id, mtga_card.name, fallback_id
                        );
                        fetch_or_cache_by_arena_id(&client, scryfall_host, &mut arena_id_cache, fallback_id).await?
                    }
                    (None, None) => None,
                };

                if let Some(json) = card_json {
                    let mut card = Card::from_json(&json);
                    card.id = mtga_card.grp_id;
                    card.set = mtga_card.expansion_code.clone();
                    cards.push(card);
                    cards_by_id.insert(mtga_card.grp_id);
                } else if get_basic_land_fallback_id(&mtga_card.name).is_some() {
                    // Last resort for basic lands: create minimal entry
                    debug!(
                        "All fetches failed for basic land '{}', using minimal card",
                        mtga_card.name
                    );
                    let mut card = Card::new(mtga_card.grp_id, &mtga_card.expansion_code, &mtga_card.name);
                    card.type_line = format!("Basic Land — {}", mtga_card.name.replace("Snow-Covered ", ""));
                    cards.push(card);
                    cards_by_id.insert(mtga_card.grp_id);
                } else {
                    warn!(
                        "Card not found in Scryfall set '{}': '{}' (number={})",
                        set_code, mtga_card.name, mtga_card.collector_number
                    );
                    failed_cards.push(mtga_card);
                }
            }
        }

        // Rate limiting between sets
        tokio::time::sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS)).await;
    }

    if !failed_cards.is_empty() {
        warn!(
            "Failed to fetch {} cards from Scryfall (likely MTGA-exclusive or very new)",
            failed_cards.len()
        );
        for card in failed_cards.iter().take(10) {
            debug!("  - {} ({}/{})", card.name, card.expansion_code, card.collector_number);
        }
        if failed_cards.len() > 10 {
            debug!("  ... and {} more", failed_cards.len() - 10);
        }
    }

    Ok(cards)
}

/// Fetch a card by arena ID, using a cache to avoid redundant Scryfall requests
async fn fetch_or_cache_by_arena_id(
    client: &reqwest::Client,
    scryfall_host: &str,
    cache: &mut HashMap<i64, serde_json::Value>,
    arena_id: i64,
) -> Result<Option<serde_json::Value>> {
    if let Some(cached) = cache.get(&arena_id) {
        debug!("Using cached Scryfall data for arena ID {}", arena_id);
        return Ok(Some(cached.clone()));
    }

    tokio::time::sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS)).await;

    if let Some(json) = fetch_scryfall_card_by_arena_id(client, scryfall_host, arena_id).await? {
        cache.insert(arena_id, json.clone());
        Ok(Some(json))
    } else {
        Ok(None)
    }
}

/// Fetch all cards from a set via Scryfall, indexed by collector number
async fn fetch_scryfall_set(
    client: &reqwest::Client,
    scryfall_host: &str,
    set: &str,
) -> Result<Option<HashMap<String, serde_json::Value>>> {
    debug!("Fetching set from Scryfall: {}", set);

    let set_query = format!("e:{set}");
    let query = vec![
        ("include_variations", "true"),
        ("order", "set"),
        ("q", &set_query),
        ("unique", "cards"),
    ];

    let response = client
        .get(format!("{scryfall_host}/cards/search"))
        .query(&query)
        .send()
        .await?;

    if response.status() == StatusCode::NOT_FOUND {
        debug!("Set '{}' returned 404 from Scryfall", set);
        return Ok(None);
    }

    response.error_for_status_ref()?;
    let mut data: serde_json::Value = response.json().await?;
    let mut cards_by_collector_number = HashMap::new();

    // Extract cards from first page
    extract_set_cards(&mut cards_by_collector_number, &data);

    // Handle pagination
    super::scryfall::paginate(
        client,
        &mut data,
        &mut cards_by_collector_number,
        Duration::from_millis(SCRYFALL_RATE_LIMIT_MS),
        extract_set_cards,
    )
    .await?;

    debug!("Fetched {} cards for set {}", cards_by_collector_number.len(), set);

    Ok(Some(cards_by_collector_number))
}

/// Extract cards from Scryfall response and index by collector number
fn extract_set_cards(cards: &mut HashMap<String, serde_json::Value>, data: &serde_json::Value) {
    if let Some(data) = data["data"].as_array() {
        for card in data {
            if let Some(collector_number) = card["collector_number"].as_str() {
                cards.insert(collector_number.to_owned(), card.clone());
            }
        }
    }
}

/// Fetch a card from Scryfall by its Arena ID
async fn fetch_scryfall_card_by_arena_id(
    client: &reqwest::Client,
    scryfall_host: &str,
    arena_id: i64,
) -> Result<Option<serde_json::Value>> {
    let url = format!("{scryfall_host}/cards/arena/{arena_id}");

    debug!("Fetching from Scryfall by Arena ID: {}", url);

    let response = client.get(&url).send().await?;

    match response.status() {
        StatusCode::OK => {
            let json = response.json().await?;
            Ok(Some(json))
        }
        StatusCode::NOT_FOUND => {
            debug!("Card not found by Arena ID: {}", arena_id);
            Ok(None)
        }
        status => {
            warn!("Unexpected status {} for Arena ID {}", status, arena_id);
            response.error_for_status_ref()?;
            Ok(None)
        }
    }
}

/// Save card collection to protobuf file
async fn save_card_collection(collection: CardCollection, output: &Path) -> Result<()> {
    let bytes = collection.encode_to_vec();
    tokio::fs::write(output, bytes).await?;
    Ok(())
}
