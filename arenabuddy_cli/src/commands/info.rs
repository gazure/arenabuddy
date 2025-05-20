use std::path::PathBuf;

use anyhow::Result;
use arenabuddy_core::proto::CardCollection;
use prost::Message;
use tracing::info;

/// Execute the Convert command based on the provided action
pub fn execute(file: &PathBuf) -> Result<()> {
    display_file_info(file)
}

/// Display information about a card data file
fn display_file_info(file: &PathBuf) -> Result<()> {
    info!("Displaying information for file: {}", file.display());

    // Read the file
    let bytes = std::fs::read(file)?;

    // Decode as CardCollection
    let collection = CardCollection::decode(bytes.as_slice())?;

    // Display information about the card data
    info!("Number of cards: {}", collection.len());

    // Additional information could be extracted here
    info!("File information displayed successfully");
    Ok(())
}
