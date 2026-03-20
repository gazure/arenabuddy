CREATE TABLE IF NOT EXISTS metagame_tournament (
    id SERIAL PRIMARY KEY,
    goldfish_id INTEGER NOT NULL UNIQUE,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    date DATE NOT NULL,
    url TEXT NOT NULL,
    scraped_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS metagame_archetype (
    id SERIAL PRIMARY KEY,
    name TEXT NOT NULL,
    format TEXT NOT NULL,
    url TEXT,
    UNIQUE (name, format)
);

CREATE TABLE IF NOT EXISTS metagame_deck (
    id SERIAL PRIMARY KEY,
    goldfish_id INTEGER NOT NULL UNIQUE,
    tournament_id INTEGER REFERENCES metagame_tournament(id),
    archetype_id INTEGER REFERENCES metagame_archetype(id),
    player_name TEXT,
    placement TEXT,
    format TEXT NOT NULL,
    date DATE,
    url TEXT NOT NULL,
    scraped_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE TABLE IF NOT EXISTS metagame_deck_card (
    id SERIAL PRIMARY KEY,
    deck_id INTEGER NOT NULL REFERENCES metagame_deck(id) ON DELETE CASCADE,
    card_name TEXT NOT NULL,
    quantity INTEGER NOT NULL,
    is_sideboard BOOLEAN NOT NULL DEFAULT FALSE
);

CREATE INDEX IF NOT EXISTS idx_metagame_deck_format ON metagame_deck (format);
CREATE INDEX IF NOT EXISTS idx_metagame_deck_archetype ON metagame_deck (archetype_id);
CREATE INDEX IF NOT EXISTS idx_metagame_deck_card_name ON metagame_deck_card (card_name);
CREATE INDEX IF NOT EXISTS idx_metagame_deck_card_deck_id ON metagame_deck_card (deck_id);
CREATE INDEX IF NOT EXISTS idx_metagame_archetype_format ON metagame_archetype (format);
CREATE INDEX IF NOT EXISTS idx_metagame_tournament_format_date ON metagame_tournament (format, date);
