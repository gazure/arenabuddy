-- Signature cards: cards that uniquely identify an archetype.
-- Derived from metagame deck data via batch analysis.
CREATE TABLE IF NOT EXISTS archetype_signature_card (
    id SERIAL PRIMARY KEY,
    archetype_id INTEGER NOT NULL REFERENCES metagame_archetype(id) ON DELETE CASCADE,
    card_name TEXT NOT NULL,
    weight REAL NOT NULL DEFAULT 1.0,
    format TEXT NOT NULL,
    computed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (archetype_id, card_name)
);

-- Materialized classification results for matches.
CREATE TABLE IF NOT EXISTS match_archetype (
    id SERIAL PRIMARY KEY,
    match_id UUID NOT NULL REFERENCES match(id) ON DELETE CASCADE,
    side TEXT NOT NULL CHECK (side IN ('controller', 'opponent')),
    archetype_id INTEGER REFERENCES metagame_archetype(id) ON DELETE SET NULL,
    archetype_name TEXT NOT NULL,
    confidence REAL NOT NULL DEFAULT 0.0,
    classified_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (match_id, side)
);

CREATE INDEX IF NOT EXISTS idx_match_archetype_match ON match_archetype (match_id);
CREATE INDEX IF NOT EXISTS idx_signature_card_format ON archetype_signature_card (format);
CREATE INDEX IF NOT EXISTS idx_signature_card_archetype ON archetype_signature_card (archetype_id);
