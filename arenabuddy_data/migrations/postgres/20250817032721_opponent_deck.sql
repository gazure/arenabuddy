CREATE TABLE IF NOT EXISTS opponent_deck (
    id SERIAL PRIMARY KEY,
    match_id UUID NOT NULL,
    cards TEXT NOT NULL,
    FOREIGN KEY (match_id) REFERENCES match(id) ON DELETE CASCADE
);

ALTER TABLE opponent_deck ADD CONSTRAINT unique_match_id UNIQUE (match_id);
