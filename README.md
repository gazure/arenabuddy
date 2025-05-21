# ArenaBuddy

An MTGA companion app

## Development Instructions

To get started with the ArenaBuddy development environment, follow these steps:

1. Install Prerequisites:

   - Rust toolchain
   - Required platform-specific dependencies for Tauri development

2. Development Commands:

   ```bash
   # Run development server
   cargo tauri dev

   # Build production version
   cargo tauri build
   ```

3. CLI Tool:

   The consolidated CLI tool provides functionality for log parsing, card scraping:

   ```bash
   # Scrape card data from online sources
   arenabuddy scrape

   # Parse MTGA log files
   arenabuddy parse --player-log /path/to/Player.log
   ```

   You can get help on any command with `arenabuddy --help` or `arenabuddy <command> --help`.

4. Project Structure:

   - `/arenabuddy_core` - common modules
   - `/public` - Static assets
   - `/arenabuddy_cli` - Consolidated command line tool for log parsing and card scraping
   - `/src` - Frontend source code
   - `/src-tauri` - Rust backend code
