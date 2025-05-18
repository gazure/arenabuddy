use std::path::PathBuf;

use anyhow::{Context, Result};
use arenabuddy_core::proto_utils;
use clap::{Parser, Subcommand};

/// A utility for working with Magic: The Gathering card data in Protocol Buffers format
#[derive(Parser)]
#[clap(author, version, about)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Convert a JSON cards file to Protocol Buffers format
    Convert {
        /// Input JSON file path
        #[clap(short, long)]
        input: PathBuf,
        
        /// Output Protocol Buffers file path
        #[clap(short, long)]
        output: PathBuf,
    },
    
    /// Display information about a Protocol Buffers cards file
    Info {
        /// Protocol Buffers file path
        #[clap(short, long)]
        file: PathBuf,
    },
}

fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt::init();
    
    // Parse command line arguments
    let cli = Cli::parse();
    
    match cli.command {
        Commands::Convert { input, output } => {
            // Convert JSON to Protocol Buffers
            proto_utils::convert_json_to_proto_file(&input, &output)
                .with_context(|| format!("Failed to convert {} to {}", input.display(), output.display()))?;
            
            println!("Successfully converted {} to {}", input.display(), output.display());
        },
        
        Commands::Info { file } => {
            // Load cards from Protocol Buffers file
            let cards = proto_utils::load_card_collection_from_file(&file)
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
                    println!("  {}. {} ({})", i+1, card.name, card.set);
                }
            }
        },
    }
    
    Ok(())
}