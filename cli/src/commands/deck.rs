//! Pretty-print Arena decklists for the CLI.
//!
//! **`deck show` input precedence** (first match wins):
//! 1. `--match-id` — load rows from Postgres `deck` for that match  
//! 2. `--clipboard` — JSON text (macOS `pbpaste`)  
//! 3. `[FILE]` — file contents, or `-` to read stdin once  
//! 4. `--main` / `--side` — literal JSON arrays of Arena IDs  
//! 5. piped stdin — when stdin is not a TTY (no `[FILE]` and no `--main`)

use std::{
    io::{self, IsTerminal, Read},
    path::Path,
};

use arenabuddy_core::{cards::CardsDatabase, display::deck::DeckDisplayRecord, models::Deck};
use arenabuddy_data::{ArenabuddyRepository, MatchDB, MetagameRepository};
use serde_json::{Map, Value};

use crate::{Error, Result};

// --- CLI wiring (passed from `lib.rs`) ---------------------------------------

/// Options collected from `arenabuddyctl deck show …`.
pub(crate) struct DeckShowOpts<'a> {
    pub cards_db: &'a Path,
    pub db_url: Option<&'a str>,
    pub match_id: Option<&'a str>,
    pub game: Option<i32>,
    pub input: Option<&'a Path>,
    pub clipboard: bool,
    pub main: Option<&'a str>,
    pub side: Option<&'a str>,
}

/// What to render after precedence rules ([`determine_input`]) resolve.
enum DeckShowInput<'a> {
    /// Postgres `deck` table for controller, optional `deck.game_number` filter.
    FromDatabase { match_id: &'a str, game: Option<i32> },
    /// Raw JSON body: either `[id,…]` or `{"deck_cards":[…],"sideboard_cards":[…]}`.
    RawJsonBody(String),
    /// `--main` / `--side` strings (already JSON arrays of `i32`).
    RawJsonCliArgs { main: &'a str, side: Option<&'a str> },
}

// --- Public entry ------------------------------------------------------------

pub async fn show(opts: DeckShowOpts<'_>) -> Result<()> {
    let catalog = CardsDatabase::new(opts.cards_db)?;

    match determine_input(&opts)? {
        DeckShowInput::FromDatabase { match_id, game } => {
            let Some(db_url) = opts.db_url else {
                return Err(Error::Invalid(
                    "--match-id requires a database URL (`--db` or ARENABUDDY_DATABASE_URL).".into(),
                ));
            };
            print_match_decks_from_db(&catalog, db_url, match_id, game).await?;
        }
        DeckShowInput::RawJsonBody(json) => {
            let (main_ids, sideboard_ids) = parse_deck_arrays_json(&json)?;
            let deck = Deck::new("Imported deck".to_string(), 0, main_ids, sideboard_ids);
            print_single_deck(&catalog, &deck, "Imported deck (JSON)");
        }
        DeckShowInput::RawJsonCliArgs { main, side } => {
            let main_ids = serde_json::from_str::<Vec<i32>>(main).map_err(crate::ParseError::Json)?;
            let sideboard_ids = match side.map(str::trim) {
                None | Some("") => Vec::new(),
                Some(slice) => serde_json::from_str::<Vec<i32>>(slice).map_err(crate::ParseError::Json)?,
            };
            let deck = Deck::new("Imported deck".to_string(), 0, main_ids, sideboard_ids);
            print_single_deck(&catalog, &deck, "Imported deck (--main / --side)");
        }
    }

    Ok(())
}

// --- Resolve which input shape we have ---------------------------------------

fn determine_input<'a>(opts: &'a DeckShowOpts<'a>) -> Result<DeckShowInput<'a>> {
    use DeckShowInput as In;

    if let Some(match_id) = opts.match_id {
        return Ok(In::FromDatabase {
            match_id,
            game: opts.game,
        });
    }

    if opts.clipboard {
        return Ok(In::RawJsonBody(read_clipboard_text()?));
    }

    if let Some(path) = opts.input {
        return Ok(In::RawJsonBody(read_file_or_stdin(path)?));
    }

    if let Some(main) = opts.main {
        return Ok(In::RawJsonCliArgs { main, side: opts.side });
    }

    if io::stdin().is_terminal() {
        return Err(Error::Invalid(
            "Provide a deck: `--match-id`, `--clipboard`, a FILE (`-` for stdin), `--main`, or pipe JSON stdin.".into(),
        ));
    }

    let mut piped = String::new();
    io::stdin().read_to_string(&mut piped)?;
    Ok(In::RawJsonBody(piped))
}

// --- Parsing: Postgres column / GRE JSON blobs ------------------------------

/// Parses JSON like the `deck_cards` / `sideboard_cards` DB columns:
/// - Bare array `[72447, 91717, …]` ⇒ mainboard only  
/// - Object with `deck_cards` and optionally `sideboard_cards` (also accepts `main` / `mainboard`, `side` / `sideboard`)
pub(crate) fn parse_deck_arrays_json(raw: &str) -> Result<(Vec<i32>, Vec<i32>)> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return Err(Error::Invalid("deck JSON was empty".into()));
    }

    let root: Value = serde_json::from_str(trimmed).map_err(crate::ParseError::Json)?;

    match root {
        Value::Array(entries) => {
            let mainboard = json_values_as_arena_ids(entries)?;
            Ok((mainboard, Vec::new()))
        }
        Value::Object(fields) => {
            let mainboard_value = alternate_field(&fields, &["deck_cards", "main", "mainboard"]);
            let sideboard_value = alternate_field(&fields, &["sideboard_cards", "side", "sideboard"]);
            Ok((
                parse_optional_id_array_field(mainboard_value, "deck_cards (mainboard)")?,
                parse_optional_id_array_field(sideboard_value, "sideboard_cards")?,
            ))
        }
        _ => Err(Error::Invalid(
            "Deck JSON must be an integer ID array or an object with deck_cards / sideboard_cards arrays.".into(),
        )),
    }
}

fn alternate_field<'a>(fields: &'a Map<String, Value>, keys: &[&str]) -> Option<&'a Value> {
    keys.iter().find_map(|k| fields.get(*k))
}

fn parse_optional_id_array_field(entry: Option<&Value>, label: &'static str) -> Result<Vec<i32>> {
    match entry {
        None => Ok(Vec::new()),
        Some(Value::Array(entries)) => json_values_as_arena_ids(entries.clone()),
        Some(_) => Err(Error::Invalid(format!(
            "`{label}` must be a JSON array of Arena card IDs."
        ))),
    }
}

fn json_values_as_arena_ids(entries: Vec<Value>) -> Result<Vec<i32>> {
    let mut ids = Vec::with_capacity(entries.len());
    for value in entries {
        let n = value
            .as_i64()
            .ok_or_else(|| Error::Invalid("Deck arrays must contain integer Arena card IDs.".into()))?;
        let id = i32::try_from(n).map_err(|_| Error::Invalid(format!("Arena ID out of range: {n}")))?;
        ids.push(id);
    }
    Ok(ids)
}

// --- IO helpers --------------------------------------------------------------

fn read_file_or_stdin(path: &Path) -> Result<String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        std::fs::read_to_string(path).map_err(Error::Io)
    }
}

#[cfg(target_os = "macos")]
fn read_clipboard_text() -> Result<String> {
    use std::process::Command;
    let output = Command::new("pbpaste").output().map_err(Error::Io)?;
    if !output.status.success() {
        return Err(Error::Invalid(
            "`pbpaste` failed. Copy deck JSON again, or save it to a file and pass that path.".into(),
        ));
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(not(target_os = "macos"))]
fn read_clipboard_text() -> Result<String> {
    Err(Error::Invalid(
        "`--clipboard` only works on macOS (uses `pbpaste`). Pipe JSON or pass a FILE instead.".into(),
    ))
}

// --- Render (`DeckDisplayRecord` + pretty_print) ----------------------------

async fn print_match_decks_from_db(
    catalog: &CardsDatabase,
    db_url: &str,
    match_id: &str,
    filter_game_number: Option<i32>,
) -> Result<()> {
    let db = MatchDB::new(Some(db_url), catalog.clone()).await?;
    db.init().await?;

    let mut decks = db.list_decklists(match_id).await?;
    if decks.is_empty() {
        return Err(Error::Invalid(format!(
            "No deck rows for match `{match_id}` (controller rows in Postgres `deck` table)."
        )));
    }
    decks.sort_by_key(Deck::game_number);

    let archetypes = db.get_match_archetypes(match_id).await;
    let (controller_label, opponent_label) = archetypes.unwrap_or((None, None));

    match filter_game_number {
        Some(game_number) => {
            let chosen = decks
                .into_iter()
                .find(|d| d.game_number() == game_number)
                .ok_or_else(|| Error::Invalid(format!("No deck row with game_number={game_number} for this match.")))?;
            let headline = deck_headline(match_id, game_number, chosen.name());
            print_one_game_with_archetypes(
                catalog,
                &chosen,
                &headline,
                controller_label.as_deref(),
                opponent_label.as_deref(),
            );
        }
        None => {
            for deck in decks {
                let game_number = deck.game_number();
                let headline = deck_headline(match_id, game_number, deck.name());
                print_one_game_with_archetypes(
                    catalog,
                    &deck,
                    &headline,
                    controller_label.as_deref(),
                    opponent_label.as_deref(),
                );
                println!();
            }
        }
    }
    Ok(())
}

fn deck_headline(match_id: &str, game_number: i32, deck_name_from_row: &str) -> String {
    format!("Match `{match_id}` · game {game_number} · controller ({deck_name_from_row})")
}

fn print_one_game_with_archetypes(
    catalog: &CardsDatabase,
    deck: &Deck,
    headline: &str,
    controller_archetype: Option<&str>,
    opponent_archetype: Option<&str>,
) {
    let mut record = DeckDisplayRecord::from_decklist(deck, catalog);

    record.archetype = match controller_archetype {
        Some(label) => label.to_owned(),
        None => "Unknown".to_owned(),
    };

    print_deck_header(headline, &record, opponent_archetype);

    let body = record.pretty_print();
    print!("{body}");
}

fn print_single_deck(catalog: &CardsDatabase, deck: &Deck, heading: &str) {
    let record = DeckDisplayRecord::from_decklist(deck, catalog);
    print_deck_header(heading, &record, None);
    let body = record.pretty_print();
    print!("{body}");
}

fn print_deck_header(title_line: &str, record: &DeckDisplayRecord, opponent_archetype: Option<&str>) {
    println!("{title_line}");
    let (main_total, side_total) = record.totals();
    println!("Cards: {main_total} main · {side_total} sideboard");

    let controller = record.archetype.as_str();
    if controller != "Unknown" && !controller.is_empty() {
        println!("Classifier archetype (controller): {controller}");
    }

    if let Some(opp) = opponent_archetype {
        println!("Classifier archetype (opponent): {opp}");
    }
    println!();
}

#[cfg(test)]
mod tests {
    use super::parse_deck_arrays_json;

    #[test]
    fn parse_plain_array_is_main_only() {
        let (main, side) = parse_deck_arrays_json("[1,2,2]").expect("parse");
        assert_eq!(main, vec![1, 2, 2]);
        assert!(side.is_empty());
    }

    #[test]
    fn parse_gre_style_object() {
        let raw = r#"{"deck_cards":[72447,91717],"sideboard_cards":[1]}"#;
        let (main, side) = parse_deck_arrays_json(raw).expect("parse");
        assert_eq!(main, vec![72447, 91717]);
        assert_eq!(side, vec![1]);
    }
}
