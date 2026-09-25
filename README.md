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
   cargo run -p arenabuddy_cli -- scrape --output ./cards.pb

   # Optionally specify a custom MTGA install path
   cargo run -p arenabuddy_cli -- scrape --mtga-path /path/to/MTGA/.../Raw --output ./cards.pb

   # Parse MTGA log files
   cargo run -p arenabuddy_cli -- parse --player-log /path/to/Player.log

   # Start interactive REPL for card searches
   cargo run -p arenabuddy_cli -- repl --cards-db ./cards.pb

   # Search cards by meaning (loads TYPESAFE_API_KEY from the environment or .env)
   SQLX_OFFLINE=true cargo run -p arenabuddy_cli -- semantic-search \
     "cheap creatures that reward casting spells" --max-mana-value 3

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

## Semantic card search

Set `TYPESAFE_API_KEY` in your environment or a Git-ignored `.env` file in the
project root. The environment variable takes precedence over the file. Run:

```bash
SQLX_OFFLINE=true cargo run -p arenabuddy_cli -- semantic-search \
  "cheap creatures that reward casting spells" --max-mana-value 3
```

The command uses the embedded card database. To use another protobuf database,
pass `--cards-db ./cards.pb`. In the card REPL, enter
`semantic cheap creatures that reward casting spells`.

Search builds a local shortlist from card names, types, keywords, and rules text,
including both faces of multiface cards. It removes duplicate printings and
expands common terms such as “ramp,” “removal,” and “reanimation.” Jev then scores
up to 60 candidates in batches of 10. Each search sends your query and candidate
card data to TypeSafe and consumes API tokens. The command reports token usage.

Use `--candidates 120` to widen the shortlist and `--limit 5` to show fewer results.
Increasing the candidate count increases API usage. Jev cannot find cards omitted
from the shortlist; try different wording if results are missing.

Use `--format standard` or `--max-mana-value 3` for exact local filters. Format
legality reflects the selected database snapshot; cards with missing legality data
are excluded when a format filter is active. A format mentioned only in the query
does not apply this filter.

Results include rules text and a relevance score from 0 to 3. Scores of at least 2
are displayed: 2 indicates a match with setup or restrictions, and 3 indicates a
direct match. This initial cutoff is a heuristic, not a calibrated guarantee.
Scores are not probabilities. A search can return no strong matches.
