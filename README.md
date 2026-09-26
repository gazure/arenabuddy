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

In the desktop app, save your TypeSafe API key in **Settings**, then open
**Cards > Search by meaning**. Enter a description and select **Search with Jev**.
The app reads the key from your OS credential store for each search. The desktop
app and CLI share the same filter interpretation and ranking logic.

The desktop search displays up to 25 results, interpreted filters, relevance
scores, and token usage. Select a result to inspect its card details. The format
override takes precedence over the query; increasing **Maximum cards to rank**
can improve coverage at the cost of more API usage. **Cancel** stops further work,
but requests already sent can still consume tokens.

For the CLI, configure credentials as follows.

Set `TYPESAFE_API_KEY` in your environment or a Git-ignored `.env` file in the
project root. The environment variable takes precedence over the file. Run:

```bash
SQLX_OFFLINE=true cargo run -p arenabuddy_cli -- semantic-search \
  "cheap creatures that reward casting spells" --max-mana-value 3
```

The command uses the embedded card database. To use another protobuf database,
pass `--cards-db ./cards.pb`. In the card REPL, enter
`semantic cheap creatures that reward casting spells`.

Search first sends your query and the supported filter definitions to Jev. It
prints the interpreted filters, then applies them to the entire local database.
For example:

```bash
SQLX_OFFLINE=true cargo run -p arenabuddy_cli -- semantic-search \
  "Standard-legal red creatures with mana value at most three, excluding defenders"
```

This first version supports one required format, one required and one excluded
card type, color inclusion and exclusion, one mana-value comparison, and one
required and one excluded keyword. Supported keywords are flying, haste, trample,
vigilance, lifelink, deathtouch, defender, reach, menace, first strike, double strike,
hexproof, indestructible, prowess, flash, and ward. Fields combine with AND.

“Red” includes red multicolor cards; “mono-red” requires exactly red. “Red or blue”
requires either color, and “red and blue” requires both. These filters use card
colors, not commander color identity. Type filters match either face. Keyword
filters use the database's keyword list. “Cheap” defaults to mana value at most 3.
Numeric candidates come from digits or number words from zero through twenty.

Queries fully covered by filters return local matches in name order after one
Jev call. `--limit` controls how many matches to display; `--candidates` does not
limit this path. Format legality and keyword membership reflect the selected
card database snapshot. Cards with missing legality data do not pass a format
filter.

Queries with additional strategic meaning, such as “cheap red creatures that
reward casting spells,” also use Jev to rank filtered candidates. When the
filtered pool exceeds `--candidates` (default: 60), local full-text search builds
a shortlist from names, types, keywords, and rules text, including both faces.
Common terms such as “ramp,” “removal,” and “reanimation” are expanded. Smaller
pools bypass this text search so wording differences cannot omit their cards.
Duplicate printings are removed before selecting candidates.

Ranking sends up to 10 cards per request. There are two dependent stages, but a
mixed query can make one interpretation request plus several ranking requests.
Each search consumes TypeSafe API tokens; successful searches report the total
usage across both stages. Use `--candidates 120` to widen the shortlist, at the
cost of more API usage. Jev cannot rank cards omitted from the shortlist.

Explicit `--format standard` and `--max-mana-value 3` flags override the respective
interpreted constraints. The printed filters include these overrides.

Mana filters support single comparisons, inclusive ranges ("between 2 and 4"),
and two exact alternatives ("costs 2 or 4"). Subjective descriptions such as
"tiny little guys" use semantic ranking based on names, creature types, stats,
and flavor text. Search cannot inspect artwork; describe card text or stats
instead of visual details.

Uncertain interpretations and compound conditions fall back to ranking. Related
strict filters are removed so they cannot prematurely exclude valid matches.
This includes multiple types or keywords and grouped alternatives. Results show
a notice when this happens; ranking still considers only the candidate shortlist.
Clear requests for unavailable data, such as prices, collection ownership, or
artwork, produce a readable explanation. Malformed API answers remain errors.
The confidence thresholds are heuristics to evaluate on real queries.

Ranked results include rules text and a relevance score from 0 to 3. Scores of
at least 2 are displayed: 2 indicates a match with setup or restrictions, and 3
indicates a direct match. This initial cutoff is a heuristic, not a calibrated
guarantee. Scores are not probabilities. A search can return no strong matches.
