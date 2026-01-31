-- Add ON DELETE CASCADE to foreign keys on deck, mulligan, and match_result tables.
-- opponent_deck already has CASCADE from its initial migration.

-- deck: drop and recreate FK
ALTER TABLE deck DROP CONSTRAINT IF EXISTS deck_match_id_fkey;
ALTER TABLE deck ADD CONSTRAINT deck_match_id_fkey
    FOREIGN KEY (match_id) REFERENCES match(id) ON DELETE CASCADE;

-- mulligan: drop and recreate FK
ALTER TABLE mulligan DROP CONSTRAINT IF EXISTS mulligan_match_id_fkey;
ALTER TABLE mulligan ADD CONSTRAINT mulligan_match_id_fkey
    FOREIGN KEY (match_id) REFERENCES match(id) ON DELETE CASCADE;

-- match_result: drop and recreate FK
ALTER TABLE match_result DROP CONSTRAINT IF EXISTS match_result_match_id_fkey;
ALTER TABLE match_result ADD CONSTRAINT match_result_match_id_fkey
    FOREIGN KEY (match_id) REFERENCES match(id) ON DELETE CASCADE;
