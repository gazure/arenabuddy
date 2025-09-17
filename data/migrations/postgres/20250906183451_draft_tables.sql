-- Draft tables for MTGA draft tracking

-- Create draft table
CREATE TABLE IF NOT EXISTS draft (
    id UUID PRIMARY KEY,
    set_code TEXT NOT NULL,
    draft_format TEXT, -- e.g., 'premier', 'quick', 'traditional'
    status TEXT DEFAULT 'in_progress', -- 'in_progress', 'completed', 'abandoned'
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    completed_at TIMESTAMP
);

-- Create draft_pack table to store each pack presented during a draft
CREATE TABLE IF NOT EXISTS draft_pack (
    id SERIAL PRIMARY KEY,
    draft_id UUID NOT NULL,
    pack_number INTEGER NOT NULL, -- 1, 2, or 3
    pick_number INTEGER NOT NULL, -- 1-15 for pack 1, 1-14 for pack 2, etc.
    cards TEXT NOT NULL, -- JSON array of card_ids presented in this pack
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (draft_id) REFERENCES draft(id) ON DELETE CASCADE,
    UNIQUE(draft_id, pack_number, pick_number)
);

-- Create draft_pick table to store which card was picked from each pack
CREATE TABLE IF NOT EXISTS draft_pick (
    id SERIAL PRIMARY KEY,
    draft_pack_id INTEGER NOT NULL,
    card_id TEXT NOT NULL, -- The card_id that was picked
    pick_time_seconds INTEGER, -- Optional: time taken to make the pick
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (draft_pack_id) REFERENCES draft_pack(id) ON DELETE CASCADE,
    UNIQUE(draft_pack_id) -- One pick per pack
);

-- Create indexes for common queries
CREATE INDEX IF NOT EXISTS draft_set_code_idx ON draft(set_code);
CREATE INDEX IF NOT EXISTS draft_status_idx ON draft(status);
CREATE INDEX IF NOT EXISTS draft_pack_draft_id_idx ON draft_pack(draft_id);
CREATE INDEX IF NOT EXISTS draft_pick_pack_id_idx ON draft_pick(draft_pack_id);
