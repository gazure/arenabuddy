CREATE TABLE IF NOT EXISTS card (
    arena_id BIGINT PRIMARY KEY,
    name TEXT NOT NULL,
    set_code TEXT NOT NULL,
    lang TEXT NOT NULL DEFAULT 'en',
    image_uri TEXT NOT NULL DEFAULT '',
    mana_cost TEXT NOT NULL DEFAULT '',
    cmc INTEGER NOT NULL DEFAULT 0,
    type_line TEXT NOT NULL DEFAULT '',
    layout TEXT NOT NULL DEFAULT 'normal',
    colors TEXT NOT NULL DEFAULT '[]',
    color_identity TEXT NOT NULL DEFAULT '[]',
    card_faces TEXT NOT NULL DEFAULT '[]',
    updated_at TIMESTAMP NOT NULL DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX IF NOT EXISTS card_name_idx ON card (name);
CREATE INDEX IF NOT EXISTS card_set_code_idx ON card (set_code);
