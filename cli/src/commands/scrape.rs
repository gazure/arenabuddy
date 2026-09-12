#![expect(clippy::too_many_lines)]
use std::{
    collections::{BTreeMap, HashMap, HashSet},
    path::{Path, PathBuf},
    time::Duration,
};

use arenabuddy_core::models::{Card, CardCollection};
use prost::Message;
use reqwest::StatusCode;
use rusqlite::Connection;
use tracing::{debug, error, info, warn};

use super::scryfall::{USER_AGENT, send_with_retry};
use crate::{Error, Result};

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

/// Execute the `Scrape` command
///
/// The output file is checkpointed after every set so that progress survives
/// an abort. With `resume`, cards already present in `output` are kept and
/// only the missing ones are fetched.
pub async fn execute(mtga_path: Option<&PathBuf>, scryfall_host: &str, output: &Path, resume: bool) -> Result<()> {
    info!("Starting MTGA database scrape...");

    let db_path = find_mtga_database(mtga_path)?;
    info!("Found MTGA database at: {}", db_path.display());

    let mtga_cards = extract_mtga_cards(&db_path)?;
    info!("Extracted {} cards from MTGA database", mtga_cards.len());

    let existing = if resume {
        load_card_collection(output).await?
    } else {
        Vec::new()
    };

    let cards = enrich_with_scryfall(mtga_cards, scryfall_host, output, existing).await?;
    info!("Saved {} cards to: {}", cards.len(), output.display());

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
///
/// `existing` seeds the result; any MTGA card whose arena ID is already
/// present is not fetched again. The collection is written to `output` after
/// each set. A set that keeps failing after retries is skipped and reported
/// at the end so one bad set does not lose the rest of the scrape.
async fn enrich_with_scryfall(
    mtga_cards: Vec<MtgaCard>,
    scryfall_host: &str,
    output: &Path,
    existing: Vec<Card>,
) -> Result<Vec<Card>> {
    let client = reqwest::Client::builder().user_agent(USER_AGENT).build()?;

    // Group MTGA cards by expansion code
    let mut cards_by_set: BTreeMap<String, Vec<MtgaCard>> = BTreeMap::new();
    for mtga_card in mtga_cards {
        cards_by_set
            .entry(mtga_card.expansion_code.clone())
            .or_default()
            .push(mtga_card);
    }

    let total_sets = cards_by_set.len();
    info!("Grouped cards into {} unique sets", total_sets);

    let mut cards_by_id: HashSet<i64> = existing.iter().map(|card| card.id).collect();
    let mut cards = existing;
    let mut failed_cards = Vec::new();
    let mut failed_sets = Vec::new();
    let mut processed_sets = 0;

    // Cache arena_id lookups to avoid repeated Scryfall fetches
    let mut arena_id_cache: HashMap<i64, serde_json::Value> = HashMap::new();

    // Process each set
    for (set_code, mtga_set_cards) in cards_by_set {
        processed_sets += 1;

        // Skip anything already in the collection (from a resumed run)
        let mtga_set_cards: Vec<MtgaCard> = mtga_set_cards
            .into_iter()
            .filter(|card| !cards_by_id.contains(&card.grp_id))
            .collect();
        if mtga_set_cards.is_empty() {
            debug!(
                "Set {}/{}: {} already complete, skipping",
                processed_sets, total_sets, set_code
            );
            continue;
        }

        info!(
            "Processing set {}/{}: {} ({} cards)",
            processed_sets,
            total_sets,
            set_code,
            mtga_set_cards.len()
        );

        // Fetch all cards from this set from Scryfall
        let scryfall_cards = match fetch_scryfall_set(&client, scryfall_host, &set_code).await {
            Ok(Some(scryfall_cards)) => scryfall_cards,
            Ok(None) => {
                warn!(
                    "Set '{}' not found in Scryfall, skipping {} cards",
                    set_code,
                    mtga_set_cards.len()
                );
                failed_cards.extend(mtga_set_cards);
                continue;
            }
            Err(err) => {
                error!("Set '{}' failed, will need a --resume run: {}", set_code, err);
                failed_sets.push(set_code);
                continue;
            }
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
                let card_json = match fetch_by_arena_id_with_fallback(
                    &client,
                    scryfall_host,
                    &mut arena_id_cache,
                    &mtga_card,
                )
                .await
                {
                    Ok(json) => json,
                    Err(err) => {
                        error!(
                            "Card '{}' ({}/{}) failed, will need a --resume run: {}",
                            mtga_card.name, set_code, mtga_card.collector_number, err
                        );
                        failed_cards.push(mtga_card);
                        continue;
                    }
                };

                if let Some(json) = card_json {
                    let mut card = Card::from_json(&json);
                    card.id = mtga_card.grp_id;
                    card.set.clone_from(&mtga_card.expansion_code);
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

        // Checkpoint progress so an abort doesn't lose completed sets
        save_card_collection(&cards, output).await?;

        // Rate limiting between sets
        tokio::time::sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS)).await;
    }

    save_card_collection(&cards, output).await?;

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

    if !failed_sets.is_empty() {
        return Err(Error::Invalid(format!(
            "{} set(s) could not be fetched from Scryfall: {}. Re-run with --resume to fill them in.",
            failed_sets.len(),
            failed_sets.join(", ")
        )));
    }

    Ok(cards)
}

/// Fetch a card by its arena ID, falling back to the canonical basic land ID
/// when the card is a basic land not found under its own ID
async fn fetch_by_arena_id_with_fallback(
    client: &reqwest::Client,
    scryfall_host: &str,
    cache: &mut HashMap<i64, serde_json::Value>,
    mtga_card: &MtgaCard,
) -> Result<Option<serde_json::Value>> {
    if let Some(json) = fetch_or_cache_by_arena_id(client, scryfall_host, cache, mtga_card.grp_id).await? {
        return Ok(Some(json));
    }
    let Some(fallback_id) = get_basic_land_fallback_id(&mtga_card.name) else {
        return Ok(None);
    };
    debug!(
        "Actual arena ID {} not found for '{}', trying fallback ID {}",
        mtga_card.grp_id, mtga_card.name, fallback_id
    );
    fetch_or_cache_by_arena_id(client, scryfall_host, cache, fallback_id).await
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
    let cards = super::scryfall::fetch_set(
        client,
        scryfall_host,
        set,
        Duration::from_millis(SCRYFALL_RATE_LIMIT_MS),
        extract_set_cards,
    )
    .await?;
    Ok(cards)
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

    let response = send_with_retry(client.get(&url)).await?;

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

/// Load the cards from an existing protobuf output file, if it exists
async fn load_card_collection(output: &Path) -> Result<Vec<Card>> {
    let bytes = match tokio::fs::read(output).await {
        Ok(bytes) => bytes,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            info!("No existing output at {}, starting fresh", output.display());
            return Ok(Vec::new());
        }
        Err(err) => return Err(err.into()),
    };
    let collection = CardCollection::decode(bytes.as_slice())
        .map_err(|err| Error::Invalid(format!("Could not decode {}: {err}", output.display())))?;
    info!(
        "Resuming with {} cards already in {}",
        collection.len(),
        output.display()
    );
    Ok(collection.cards)
}

/// Save cards to the protobuf output file
///
/// Writes to a temporary sibling file and renames it into place so a crash
/// mid-write never leaves a truncated output behind.
async fn save_card_collection(cards: &[Card], output: &Path) -> Result<()> {
    let bytes = CardCollection::with_cards(cards.to_vec()).encode_to_vec();
    let tmp = output.with_extension("pb.tmp");
    tokio::fs::write(&tmp, bytes).await?;
    tokio::fs::rename(&tmp, output).await?;
    Ok(())
}
