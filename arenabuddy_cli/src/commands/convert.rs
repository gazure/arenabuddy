use std::path::PathBuf;

use anyhow::Result;
use tracing::info;
use arenabuddy_core::proto_utils;

use super::definitions::ConvertAction;

/// Execute the Convert command based on the provided action
pub fn execute(action: &ConvertAction) -> Result<()> {
    match action {
        ConvertAction::JsonToProto { input, output } => convert_json_to_proto(input, output),
        ConvertAction::Info { file } => display_file_info(file),
    }
}

/// Convert JSON card data to Protocol Buffers format
fn convert_json_to_proto(input: &PathBuf, output: &PathBuf) -> Result<()> {
    info!("Converting JSON from {} to Protocol Buffers at {}", input.display(), output.display());
    // Create parent directories if they don't exist
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }

    // Serialize Protocol Buffers to file
    // Use proto_utils to write the converted data
    proto_utils::convert_json_to_proto_file(input, output)?;

    info!("Conversion completed successfully");
    Ok(())
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
