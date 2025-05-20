use std::path::PathBuf;

use anyhow::Result;
use arenabuddy_core::proto_utils;
use tracing::info;

/// Execute the Convert command based on the provided action
pub fn execute(file: &PathBuf) -> Result<()> {
    display_file_info(&file)
}

/// Display information about a card data file
fn display_file_info(file: &PathBuf) -> Result<()> {
    info!("Displaying information for file: {}", file.display());

    // Read and analyze the Protocol Buffers file using proto_utils
    let cards = proto_utils::load_card_collection_from_file(file)?;

    // Display information about the card data
    info!("Number of cards: {}", cards.len());

    // Additional information could be extracted here
    info!("File information displayed successfully");
    Ok(())
}
