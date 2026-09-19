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
        let mut mtga_set_cards: Vec<MtgaCard> = mtga_set_cards
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
                    "Set '{}' not found in Scryfall ({} cards); trying fallbacks for missing collector numbers",
                    set_code,
                    mtga_set_cards.len()
                );
                let (without_number, other_cards) = mtga_set_cards
                    .into_iter()
                    .partition(|card| card.collector_number == "0");
                failed_cards.extend(other_cards);
                mtga_set_cards = without_number;
                if mtga_set_cards.is_empty() {
                    continue;
                }
                HashMap::new()
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

            // Zero is a placeholder, so it must never select a printing by number.
            if mtga_card.collector_number == "0" {
                match resolve_without_collector_number(
                    &client,
                    scryfall_host,
                    &mut arena_id_cache,
                    &scryfall_cards,
                    &mtga_card,
                )
                .await
                {
                    Ok(Some(json)) => {
                        let mut card = Card::from_json(&json);
                        card.id = mtga_card.grp_id;
                        cards.push(card);
                        cards_by_id.insert(mtga_card.grp_id);
                        continue;
                    }
                    Ok(None) if get_basic_land_fallback_id(&mtga_card.name).is_some() => {
                        let mut card = Card::new(mtga_card.grp_id, &mtga_card.expansion_code, &mtga_card.name);
                        card.type_line = format!("Basic Land — {}", mtga_card.name.replace("Snow-Covered ", ""));
                        cards.push(card);
                        cards_by_id.insert(mtga_card.grp_id);
                        continue;
                    }
                    Ok(None) => {
                        warn!("Card not found: '{}' (arena_id={})", mtga_card.name, mtga_card.grp_id);
                        failed_cards.push(mtga_card);
                        continue;
                    }
                    Err(err) => {
                        error!(
                            "Card '{}' (arena_id={}) failed: {}",
                            mtga_card.name, mtga_card.grp_id, err
                        );
                        failed_cards.push(mtga_card);
                        continue;
                    }
                }
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

// Exact face names also identify the primary face of a multifaced card.
fn matches_card_name(json: &serde_json::Value, name: &str) -> bool {
    json["name"].as_str() == Some(name)
        || json["card_faces"]
            .as_array()
            .and_then(|faces| faces.first())
            .and_then(|face| face["name"].as_str())
            == Some(name)
}

async fn resolve_without_collector_number(
    client: &reqwest::Client,
    scryfall_host: &str,
    cache: &mut HashMap<i64, serde_json::Value>,
    set_cards: &HashMap<String, serde_json::Value>,
    mtga_card: &MtgaCard,
) -> Result<Option<serde_json::Value>> {
    if let Some(json) = fetch_or_cache_by_arena_id(client, scryfall_host, cache, mtga_card.grp_id).await?
        && matches_card_name(&json, &mtga_card.name)
    {
        return Ok(Some(json));
    }

    // Sort matching printings so HashMap iteration cannot change the selected artwork.
    let set_match = set_cards
        .iter()
        .filter(|(_, json)| matches_card_name(json, &mtga_card.name))
        .min_by(|(number_a, _), (number_b, _)| number_a.cmp(number_b))
        .map(|(_, json)| json.clone());
    let json = if let Some(json) = set_match {
        Some(json)
    } else {
        tokio::time::sleep(Duration::from_millis(SCRYFALL_RATE_LIMIT_MS)).await;
        let response = send_with_retry(
            client
                .get(format!("{scryfall_host}/cards/named"))
                .query(&[("exact", mtga_card.name.as_str())]),
        )
        .await?;
        if response.status() == StatusCode::NOT_FOUND {
            None
        } else {
            response.error_for_status_ref()?;
            let json: serde_json::Value = response.json().await?;
            // Reject normalization to a different card, including a rebalanced version.
            matches_card_name(&json, &mtga_card.name).then_some(json)
        }
    };

    if let Some(json) = &json {
        // Keep the source printing's set, number, and artwork together. Only the Arena ID changes.
        info!(
            "Using name fallback for '{}' (arena_id={}, MTGA set={}): Scryfall {}/{}",
            mtga_card.name, mtga_card.grp_id, mtga_card.expansion_code, json["set"], json["collector_number"]
        );
    }
    Ok(json)
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

#[cfg(test)]
mod tests {
    use serde_json::{Value, json};
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    use super::*;

    async fn mock_scryfall(responses: Vec<(&str, u16, Value)>) -> (String, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("http://{}", listener.local_addr().unwrap());
        let responses: Vec<_> = responses
            .into_iter()
            .map(|(path, status, body)| (path.to_owned(), status, body))
            .collect();
        let task = tokio::spawn(async move {
            for (path, status, body) in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                loop {
                    let mut buf = [0; 1024];
                    let read = socket.read(&mut buf).await.unwrap();
                    assert!(read > 0);
                    request.extend_from_slice(&buf[..read]);
                    if request.windows(4).any(|window| window == b"\r\n\r\n") {
                        break;
                    }
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with(&format!("GET {path} ")), "{request}");
                let body = body.to_string();
                let response = format!(
                    "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (host, task)
    }

    fn mtga(id: i64, set: &str, name: &str) -> MtgaCard {
        MtgaCard {
            grp_id: id,
            expansion_code: set.into(),
            collector_number: "0".into(),
            name: name.into(),
        }
    }

    fn printing(name: &str, set: &str, number: &str) -> Value {
        json!({"name": name, "set": set, "collector_number": number, "lang": "en"})
    }

    #[tokio::test]
    async fn recovers_reported_cards_and_checkpoints_source_printings() {
        let bauble = printing("Urza's Bauble", "ana", "36");
        let twist = printing("Mind Twist", "spg", "166");
        let ravager = printing("Arcbound Ravager", "pza", "14");
        let library = printing("Sylvan Library", "spg", "155");
        let (host, server) = mock_scryfall(vec![
            (
                "/cards/search?include_variations=true&order=set&q=e%3AANA&unique=cards",
                200,
                json!({"data": [bauble, printing("Wrong card", "ana", "0")]}),
            ),
            ("/cards/arena/101029", 404, json!({})),
            ("/cards/named?exact=Mind+Twist", 200, twist),
            ("/cards/arena/101030", 404, json!({})),
            (
                "/cards/search?include_variations=true&order=set&q=e%3APZA&unique=cards",
                200,
                json!({"data": [ravager]}),
            ),
            ("/cards/arena/100676", 404, json!({})),
            (
                "/cards/search?include_variations=true&order=set&q=e%3ASPG&unique=cards",
                200,
                json!({"data": [library]}),
            ),
            ("/cards/arena/102832", 404, json!({})),
        ])
        .await;
        let output = std::env::temp_dir().join(format!("arenabuddy-scrape-{}.pb", std::process::id()));
        let cards = enrich_with_scryfall(
            vec![
                mtga(101_029, "ANA", "Mind Twist"),
                mtga(101_030, "ANA", "Urza's Bauble"),
                mtga(100_676, "PZA", "Arcbound Ravager"),
                mtga(102_832, "SPG", "Sylvan Library"),
            ],
            &host,
            &output,
            Vec::new(),
        )
        .await
        .unwrap();
        server.await.unwrap();
        let actual: Vec<_> = cards
            .iter()
            .map(|c| (c.id, c.name.as_str(), c.set.as_str(), c.collector_number.as_str()))
            .collect();
        assert_eq!(
            actual,
            vec![
                (101_029, "Mind Twist", "spg", "166"),
                (101_030, "Urza's Bauble", "ana", "36"),
                (100_676, "Arcbound Ravager", "pza", "14"),
                (102_832, "Sylvan Library", "spg", "155"),
            ]
        );
        assert_eq!(load_card_collection(&output).await.unwrap(), cards);
        // A resumed scrape must keep recovered IDs without requesting metadata again.
        let resumed = enrich_with_scryfall(vec![mtga(101_029, "ANA", "Mind Twist")], &host, &output, cards.clone())
            .await
            .unwrap();
        assert_eq!(resumed, cards);
        tokio::fs::remove_file(output).await.unwrap();
    }

    #[tokio::test]
    async fn prefers_arena_id_and_rejects_inexact_name_fallback() {
        let card = mtga(101_030, "ANA", "Urza's Bauble");
        let exact = printing("Urza's Bauble", "ana", "99");
        let (host, server) = mock_scryfall(vec![("/cards/arena/101030", 200, exact.clone())]).await;
        let client = reqwest::Client::new();
        let set = HashMap::from([("36".into(), printing("Urza's Bauble", "ana", "36"))]);
        let resolved = resolve_without_collector_number(&client, &host, &mut HashMap::new(), &set, &card)
            .await
            .unwrap();
        assert_eq!(resolved, Some(exact));
        server.await.unwrap();

        let (host, server) = mock_scryfall(vec![
            ("/cards/arena/101030", 404, json!({})),
            (
                "/cards/named?exact=Urza%27s+Bauble",
                200,
                printing("A-Urza's Bauble", "ana", "36"),
            ),
        ])
        .await;
        assert!(
            resolve_without_collector_number(&client, &host, &mut HashMap::new(), &HashMap::new(), &card)
                .await
                .unwrap()
                .is_none()
        );
        server.await.unwrap();
    }

    #[tokio::test]
    async fn unknown_set_still_uses_name_fallback_and_missing_names_stay_missing() {
        let (host, server) = mock_scryfall(vec![
            (
                "/cards/search?include_variations=true&order=set&q=e%3AUNKNOWN&unique=cards",
                404,
                json!({}),
            ),
            ("/cards/arena/1", 404, json!({})),
            (
                "/cards/named?exact=Mind+Twist",
                200,
                printing("Mind Twist", "spg", "166"),
            ),
            ("/cards/arena/2", 404, json!({})),
            ("/cards/named?exact=Unknown+Card", 404, json!({})),
        ])
        .await;
        let output = std::env::temp_dir().join(format!("arenabuddy-scrape-unknown-{}.pb", std::process::id()));
        let mut numbered = mtga(3, "UNKNOWN", "Numbered Card");
        numbered.collector_number = "1".into();
        let cards = enrich_with_scryfall(
            vec![
                mtga(1, "UNKNOWN", "Mind Twist"),
                mtga(2, "UNKNOWN", "Unknown Card"),
                numbered,
            ],
            &host,
            &output,
            Vec::new(),
        )
        .await
        .unwrap();
        server.await.unwrap();
        assert_eq!(cards.len(), 1);
        assert_eq!(cards[0].id, 1);
        assert_eq!(cards[0].set, "spg");
        tokio::fs::remove_file(output).await.unwrap();
    }

    #[test]
    fn matches_only_full_name_or_primary_face() {
        let card = json!({"name": "Front // Back", "card_faces": [{"name": "Front"}, {"name": "Back"}]});
        assert!(matches_card_name(&card, "Front"));
        assert!(matches_card_name(&card, "Front // Back"));
        assert!(!matches_card_name(&card, "Back"));
        assert!(!matches_card_name(&card, "A-Front"));
    }
}
