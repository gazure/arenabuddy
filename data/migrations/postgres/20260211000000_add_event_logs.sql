CREATE TABLE match_event_log (
    match_id UUID NOT NULL,
    game_number INTEGER NOT NULL,
    events_json TEXT NOT NULL,
    PRIMARY KEY (match_id, game_number),
    FOREIGN KEY (match_id) REFERENCES match(id) ON DELETE CASCADE
);
