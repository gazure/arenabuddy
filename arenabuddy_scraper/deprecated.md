# DEPRECATED: ArenabudAy Scraper

This tool has been deprecated and its functionality has been merged into the consolidated `arenabuddy` CLI tool.

## Why Was This Tool Deprecated?

Having two separate CLI tools (`abscraper` and `arenaparser`) for the ArenabudAy project was causing confusion and maintenance overhead. To provide a better developer experience, we've merged the functionality into a single consolidated CLI.

## How To Use The New Consolidated CLI

The new CLI provides all the functionality of the old scraper, with improved command structure and maintainability.

### Installation

```bash
# From the project root
make install
```

### Scraping Card Data

```bash
# Equivalent to the old "abscraper scrape" command
arenabuddy scrape

# With custom options
arenabuddy scrape --output-dir custom_dir --scryfall-host https://custom-url
```

### Processing Card Data

```bash
# Equivalent to the old "abscraper process" command
arenabuddy process

# With custom options
arenabuddy process --scryfall-cards-file path/to/cards.json --seventeen-lands-file path/to/data.csv
```

### Cleaning Data

```bash
# Equivalent to the old "abscraper clean" command
arenabuddy clean
```

### Getting Help

```bash
# Get general help
arenabuddy --help

# Get help for a specific command
arenabuddy scrape --help
```

## Benefits of the Consolidated CLI

- Single tool to learn and use
- Consistent command structure
- Easier maintenance
- Better integration between related commands

## Timeline

This scraper tool will be completely removed in a future release. Please migrate to the new CLI as soon as possible.