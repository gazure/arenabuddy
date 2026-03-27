# ArenaBuddy

An MTGA companion app

## Development Instructions

To get started with the ArenaBuddy development environment, follow these steps:

1. Install Prerequisites:

   - Rust toolchain
   - Required platform-specific dependencies for dioxus development

2. Development Commands:

   ```bash
   dx serve --platform desktop
   ```

3. CLI Tool:

   The consolidated CLI tool (`arenabuddyctl`) provides functionality for log parsing, card scraping, and more:

   ```bash
   # Scrape card data from local MTGA database + Scryfall enrichment
   # (auto-detects MTGA install path on macOS/Windows/Linux)
   cargo run -p arenabuddy_cli -- scrape-mtga --output ./cards.pb

   # Optionally specify a custom MTGA install path
   cargo run -p arenabuddy_cli -- scrape-mtga --mtga-path /path/to/MTGA/.../Raw --output ./cards.pb

   # Scrape card data from online sources (17Lands + Scryfall)
   cargo run -p arenabuddy_cli -- scrape --output ./cards.pb

   # Parse MTGA log files
   cargo run -p arenabuddy_cli -- parse --player-log /path/to/Player.log

   # Start interactive REPL for card searches
   cargo run -p arenabuddy_cli -- repl --cards-db ./cards.pb

   # Generate structured event log from a Player.log
   cargo run -p arenabuddy_cli -- event-log --player-log /path/to/Player.log
   ```

   You can get help on any command with `cargo run -p arenabuddy_cli -- --help` or `cargo run -p arenabuddy_cli -- <command> --help`.

4. Project Structure:

   - `/core` - common modules
   - `/cli` - Consolidated command line tool for log parsing and card scraping
   - `/data` - data layer
   - `/arenabuddy` - Dioxus desktop app
   - `/server` - gRPC backend service
   - `/web` - Web splash page
   - `/metagame` - Metagame scraping and deck classification
