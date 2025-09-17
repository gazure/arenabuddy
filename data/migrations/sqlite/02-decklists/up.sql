CREATE TABLE IF NOT EXISTS decks (
    match_id TEXT,
    game_number INTEGER,
    deck_cards TEXT,
    sideboard_cards TEXT,
    PRIMARY KEY (match_id, game_number),
    FOREIGN KEY (match_id) REFERENCES matches(id)
)
