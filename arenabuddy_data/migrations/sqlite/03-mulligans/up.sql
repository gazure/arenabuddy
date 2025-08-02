CREATE TABLE IF NOT EXISTS mulligans (
    id INTEGER PRIMARY KEY,
    match_id TEXT,
    game_number INTEGER,
    number_to_keep INTEGER,
    hand TEXT,
    play_draw TEXT,
    opponent_identity TEXT,
    decision TEXT,
    FOREIGN KEY (match_id) REFERENCES matches(id)
)
