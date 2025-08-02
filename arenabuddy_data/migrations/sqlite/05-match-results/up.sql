CREATE TABLE IF NOT EXISTS match_results
(
    match_id TEXT PRIMARY KEY,
    result_scope TEXT,
    winning_team_id INTEGER,
    game_number INTEGER,
    FOREIGN KEY (match_id) REFERENCES matches(id)
);
CREATE UNIQUE INDEX match_game_number_idx ON match_results (`match_id`, `game_number`) WHERE `game_number` IS NOT NULL;
