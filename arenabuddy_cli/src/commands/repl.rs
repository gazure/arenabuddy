use std::{collections::HashMap, path::Path};

use anyhow::Result;
use arenabuddy_core::cards::CardsDatabase;
use rustyline::DefaultEditor;

/// Execute the REPL command
///
/// # Errors
///
/// Will return an error if the cards database cannot be loaded or if there's an I/O error
pub fn execute(cards_db_path: &Path) -> Result<()> {
    // Load the cards database
    let cards_db = CardsDatabase::new(cards_db_path)?;
    println!("Loaded {} cards", cards_db.db.len());

    let mut rl = DefaultEditor::new()?;
    println!("Arenabuddy REPL");
    println!("Available commands:");
    println!("  find <arena_id> - Find a card by Arena ID");
    println!("  count [set_code] - Count cards, optionally filtered by set code");
    println!("  sets - List all set codes");
    println!("  info - Display information about the loaded db file");
    println!("  help - Show this help message");
    println!("  exit - Exit the REPL");

    loop {
        let readline = rl.readline(">> ");
        match readline {
            Ok(line) => {
                rl.add_history_entry(&line)?;

                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.is_empty() {
                    continue;
                }

                match parts[0] {
                    "find" => {
                        if parts.len() < 2 {
                            println!("Usage: find <arena_id>");
                            continue;
                        }

                        match parts[1].parse::<i64>() {
                            Ok(arena_id) => {
                                find_card(&cards_db, arena_id);
                            }
                            Err(_) => {
                                println!("Invalid Arena ID: {}", parts[1]);
                            }
                        }
                    }
                    "count" => {
                        if parts.len() > 1 {
                            count_cards_by_set(&cards_db, Some(parts[1]));
                        } else {
                            count_cards_by_set(&cards_db, None);
                        }
                    }
                    "sets" => {
                        list_sets(&cards_db);
                    }
                    "info" => display_file_info(&cards_db),
                    "help" => {
                        println!("Available commands:");
                        println!("  find <arena_id> - Find a card by Arena ID");
                        println!(
                            "  count [set_code] - Count cards, optionally filtered by set code"
                        );
                        println!("  sets - List all set codes");
                        println!("  info <file> - Display information about a card data file");
                        println!("  help - Show this help message");
                        println!("  exit - Exit the REPL");
                    }
                    "exit" | "quit" => {
                        println!("Goodbye!");
                        break;
                    }
                    _ => {
                        println!("Unknown command: {}", parts[0]);
                        println!("Type 'help' for available commands");
                    }
                }
            }
            Err(err) => {
                println!("Error: {err}");
                break;
            }
        }
    }

    Ok(())
}

fn find_card(cards_db: &CardsDatabase, arena_id: i64) {
    match cards_db.get(&arena_id.to_string()) {
        Some(card) => {
            println!("Card found:");
            println!("  ID: {}", card.id);
            println!("  Name: {}", card.name);
            println!("  Set: {}", card.set);
            println!("  Type: {}", card.type_line);
            if !card.mana_cost.is_empty() {
                println!("  Mana Cost: {}", card.mana_cost);
            }
            println!("  CMC: {}", card.cmc);
        }
        None => {
            println!("No card found with Arena ID: {arena_id}");
        }
    }
}

fn count_cards_by_set(cards_db: &CardsDatabase, filter_set: Option<&str>) {
    let mut set_counts: HashMap<String, usize> = HashMap::new();

    for card in cards_db.db.values() {
        if let Some(filter) = filter_set {
            if card.set.to_lowercase() == filter.to_lowercase() {
                *set_counts.entry(card.set.clone()).or_insert(0) += 1;
            }
        } else {
            *set_counts.entry(card.set.clone()).or_insert(0) += 1;
        }
    }

    if set_counts.is_empty() {
        if let Some(filter) = filter_set {
            println!("No cards found for set code: {filter}");
        } else {
            println!("No cards found in the database");
        }
        return;
    }

    // Sort by set code
    let mut sets: Vec<(String, usize)> = set_counts.into_iter().collect();
    sets.sort_by(|a, b| a.0.cmp(&b.0));

    if let Some(filter) = filter_set {
        if sets.len() == 1 {
            let (set, count) = &sets[0];
            println!("Set {set}: {count} cards");
        } else {
            println!("Filtered results for set '{filter}' not found.");
        }
    } else {
        println!("Cards by set:");
        let total: usize = sets.iter().map(|(_, count)| count).sum();
        for (set, count) in &sets {
            println!("  {set}: {count} cards");
        }
        println!("Total: {total} cards in {} sets", sets.len());
    }
}

fn list_sets(cards_db: &CardsDatabase) {
    let mut sets = std::collections::HashSet::new();

    for card in cards_db.db.values() {
        sets.insert(card.set.clone());
    }

    let mut sets_vec: Vec<String> = sets.into_iter().collect();
    sets_vec.sort();

    println!("Available set codes ({}):", sets_vec.len());
    for set in sets_vec {
        println!("  {set}");
    }
}

/// Display information about a card data file
fn display_file_info(db: &CardsDatabase) {
    // Display information about the card data
    println!("Number of cards: {}", db.len());
}
