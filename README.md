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

3. Project Structure:

   - `/arenabuddy_scraper` - scraping utility for building card databases from external sources
   - `/public` - Static assets
   - `/arenabuddy_cli` - Command line tool for testing arena log parsing without UI
   - `/src` - Frontend source code
   - `/src-tauri` - Rust backend code
