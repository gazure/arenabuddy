-- Run with psql from the directory containing the reviewed match-owners.csv.
-- By default, validate and preview the assignments, then roll them back.
\set ON_ERROR_STOP on
\if :{?apply}
\else
  \set apply false
\endif

BEGIN;
SET LOCAL lock_timeout = '10s';
CREATE TEMP TABLE match_owner_assignment (
    match_id UUID PRIMARY KEY,
    user_id UUID NOT NULL
) ON COMMIT DROP;
\copy match_owner_assignment FROM 'match-owners.csv' WITH (FORMAT csv, HEADER true)

-- Exclude concurrent match writes while validating and assigning ownership.
LOCK TABLE match IN SHARE ROW EXCLUSIVE MODE;

DO $$
BEGIN
    IF EXISTS (
        SELECT 1 FROM match_owner_assignment a
        LEFT JOIN match m ON m.id = a.match_id
        WHERE m.id IS NULL
    ) THEN
        RAISE EXCEPTION 'Assignment contains an unknown match ID';
    END IF;
    IF EXISTS (
        SELECT 1 FROM match_owner_assignment a
        LEFT JOIN app_user u ON u.id = a.user_id
        WHERE u.id IS NULL
    ) THEN
        RAISE EXCEPTION 'Assignment contains an unknown user ID';
    END IF;
    IF EXISTS (
        SELECT 1 FROM match_owner_assignment a
        JOIN match m ON m.id = a.match_id
        WHERE m.user_id IS NOT NULL AND m.user_id <> a.user_id
    ) THEN
        RAISE EXCEPTION 'Assignment would replace an existing owner';
    END IF;
END
$$;

UPDATE match m
SET user_id = a.user_id
FROM match_owner_assignment a
WHERE m.id = a.match_id AND m.user_id IS NULL
RETURNING m.id, m.user_id, m.controller_player_name, m.created_at;

SELECT count(*) AS remaining_unowned_matches FROM match WHERE user_id IS NULL;

\if :apply
    COMMIT;
\else
    ROLLBACK;
    \echo 'Preview only. Run again with -v apply=true to commit the reviewed assignments.'
\endif
