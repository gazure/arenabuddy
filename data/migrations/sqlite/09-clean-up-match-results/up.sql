DELETE FROM match_results WHERE rowid NOT IN (
    SELECT min(rowid) FROM match_results GROUP BY match_id, game_number
);

UPDATE match_results SET game_number = 0 WHERE game_number IS NULL;
