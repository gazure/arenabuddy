# Protocol Buffers for ArenaDesk

This directory contains Protocol Buffer definitions for the ArenaDesk application. Protocol Buffers provide an efficient binary serialization format that is language-agnostic and backward-compatible.

## Card Definitions

The main proto file is `card.proto`, which defines the structure for Magic: The Gathering cards. These definitions closely match the structure of cards in the JSON format but are optimized for binary serialization.

## Usage in Rust

The Protocol Buffers definitions are compiled into Rust code at build time using the `prost` and `prost-build` crates. The generated code is included in the `arenabuddy_core` crate.

### Converting Between Formats

The `arenabuddy_core::proto_utils` module provides utility functions for converting between JSON and Protocol Buffers formats:

```rust
use arenabuddy_core::proto_utils;

// Convert JSON file to Protocol Buffers
proto_utils::convert_json_to_proto_file("cards.json", "cards.pb")?;

// Load cards from Protocol Buffers file
let cards = proto_utils::load_card_collection_from_file("cards.pb")?;

// Save cards to Protocol Buffers file
proto_utils::save_card_collection_to_file(&cards, "cards.pb")?;
```

### Command-line Utility

The `cardconverter` command-line utility in the `arenabuddy_cli` crate demonstrates the Protocol Buffers functionality:

```bash
# Convert JSON to Protocol Buffers
cargo run --bin cardconverter -- convert --input data/cards-examples.json --output data/cards.pb

# Display information about a Protocol Buffers file
cargo run --bin cardconverter -- info --file data/cards.pb
```

## Benefits of Using Protocol Buffers

1. **Smaller Size**: Protocol Buffers typically produce smaller serialized data compared to JSON or XML.
2. **Faster Parsing**: Binary format is more efficient to parse than text-based formats.
3. **Schema Enforcement**: The defined schema ensures data consistency.
4. **Forward/Backward Compatibility**: Protocol Buffers are designed to handle evolving schemas.
5. **Cross-Language Support**: Generated code is available for many programming languages.

## Protocol Buffer Definitions

The `card.proto` file defines the following message types:

- `Card`: Represents a single Magic: The Gathering card
- `CardFace`: Represents a single face of a split or multi-faced card
- `CardCollection`: A collection of cards

## Future Work

Future improvements could include:
- Adding more specific MTG-related fields to the schema
- Performance optimizations for large card collections
- Additional utility functions for filtering and searching cards