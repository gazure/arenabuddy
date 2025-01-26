ALTER TABLE match_results RENAME TO match_results_old;

CREATE TABLE match_results
(
    match_id TEXT,
    game_number INTEGER,
    result_scope TEXT,
    winning_team_id INTEGER,
    PRIMARY KEY (match_id, game_number),
    FOREIGN KEY (match_id) REFERENCES matches(id)
);