#![expect(clippy::too_many_lines)]
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    thread::sleep,
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
            std::env::home_dir().ok_or_else(|| Error::Config("Could not determine home directory".to_string()))?;

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
            && path.extension().is_some_and(|ext| ext.eq_ignore_ascii_case("rs"))
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

    // Cache basic land data to avoid repeated fetches
    let mut basic_land_cache: HashMap<i64, serde_json::Value> = HashMap::new();

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
            } else if let Some(fallback_id) = get_basic_land_fallback_id(&mtga_card.name) {
                // For basic lands, fetch the canonical card from Scryfall for metadata/images
                // Use cache to avoid repeated fetches
                let basic_land_json = if let Some(cached) = basic_land_cache.get(&fallback_id) {
                    debug!(
                        "Using cached basic land data for '{}' (fallback ID: {})",
                        mtga_card.name, fallback_id
                    );
                    Some(cached.clone())
                } else {
                    debug!(
                        "Fetching canonical basic land data for '{}' (fallback ID: {}, actual ID: {})",
                        mtga_card.name, fallback_id, mtga_card.grp_id
                    );

                    // Rate limit before the request
                    sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS));

                    // Fetch and cache
                    if let Some(json) = fetch_scryfall_card_by_arena_id(&client, scryfall_host, fallback_id).await? {
                        basic_land_cache.insert(fallback_id, json.clone());
                        Some(json)
                    } else {
                        None
                    }
                };

                if let Some(json) = basic_land_json {
                    let mut card = Card::from_json(&json);
                    // Override with MTGA's actual GrpId so each variant has unique ID
                    card.id = mtga_card.grp_id;
                    // Update the set to match MTGA's set
                    card.set = mtga_card.expansion_code.clone();

                    cards.push(card);
                    cards_by_id.insert(mtga_card.grp_id);
                } else {
                    // If even the fallback fetch fails, create minimal entry
                    debug!("Fallback fetch failed for {}, using minimal card", mtga_card.name);
                    let mut card = Card::new(mtga_card.grp_id, &mtga_card.expansion_code, &mtga_card.name);
                    card.type_line = format!("Basic Land — {}", mtga_card.name.replace("Snow-Covered ", ""));
                    cards.push(card);
                    cards_by_id.insert(mtga_card.grp_id);
                }
            } else {
                warn!(
                    "Card not found in Scryfall set '{}': '{}' (number={})",
                    set_code, mtga_card.name, mtga_card.collector_number
                );
                failed_cards.push(mtga_card);
            }
        }

        // Rate limiting between sets
        sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS));
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
    while let Some(next_page) = data["next_page"].as_str() {
        sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS));

        let response = client.get(next_page).send().await?;
        response.error_for_status_ref()?;
        data = response.json().await?;
        extract_set_cards(&mut cards_by_collector_number, &data);
    }

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
