-- Initial PostgreSQL migration for ArenaData
-- Consolidated from SQLite migrations 01-09

-- Create matches table
CREATE TABLE IF NOT EXISTS match (
    id UUID PRIMARY KEY,
    controller_seat_id INTEGER NOT NULL,
    controller_player_name TEXT NOT NULL,
    opponent_player_name TEXT NOT NULL,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Create decks table
CREATE TABLE IF NOT EXISTS deck (
    match_id UUID,
    game_number INTEGER NOT NULL,
    deck_cards TEXT NOT NULL,
    sideboard_cards TEXT NOT NULL,
    PRIMARY KEY (match_id, game_number),
    FOREIGN KEY (match_id) REFERENCES match(id)
);

-- Create mulligans table
CREATE TABLE IF NOT EXISTS mulligan (
    id SERIAL PRIMARY KEY,
    match_id UUID,
    game_number INTEGER NOT NULL,
    number_to_keep INTEGER NOT NULL,
    hand TEXT NOT NULL,
    play_draw TEXT NOT NULL,
    opponent_identity TEXT NOT NULL,
    decision TEXT NOT NULL,
    FOREIGN KEY (match_id) REFERENCES match(id)
);

-- Create match_results table
CREATE TABLE IF NOT EXISTS match_result (
    match_id UUID,
    game_number INTEGER NOT NULL,
    result_scope TEXT NOT NULL,
    winning_team_id INTEGER NOT NULL,
    PRIMARY KEY (match_id, game_number),
    FOREIGN KEY (match_id) REFERENCES match(id)
);

-- Create indexes
CREATE UNIQUE INDEX IF NOT EXISTS match_game_hand_idx ON mulligan (match_id, game_number, number_to_keep);
