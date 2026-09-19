# Upgrade match ownership enforcement

This release changes server authorization without changing the gRPC protocol or
database schema. Existing authenticated clients can continue to upload, list,
download, classify, and delete their own matches. No client upgrade or token reset
is required for this change.

The server rejects an upload if the match ID already belongs to another user or
has no assigned owner. It returns `PERMISSION_DENIED` before changing the match or
its child records. Existing owners never change through an upload. Local desktop
databases can still insert and update matches with no owner.

## Plan the rollout

The deployment workflow in `.github/workflows/deploy-server.yml` automatically
deploys changes on `main` that touch the server or data crates. Complete the backup
and ownership review before merging, or temporarily disable that workflow in
GitHub Actions. Test this release against a restored backup before production.

No new SQLx migration is needed: the existing `user_id` column supports the check.
Do not edit previously applied migration files. The server still runs all pending
migrations on startup, so compare the deployed revision with this release for
unrelated schema changes if the droplet is behind.

Matches retain their IDs, owners, and child records. Rows with a null `user_id`
remain stored, but authenticated clients cannot claim them by uploading. Review
and assign those rows explicitly if their owners are known. Leave uncertain rows
unassigned. Player names alone are not proof of ownership.

The schema stores one owner's perspective per match ID. If two legitimate users
upload the same Arena match ID, the second receives a conflict. Supporting both
perspectives requires a separate schema and API design; this release preserves
the existing owner's data.

## Back up and inspect production

Run the following commands on the droplet from the deployed `server` directory.
They assume the PostgreSQL service and database names in `docker-compose.yml`.
Keep backups and exported account data private.

1. Record the running server image and retain that image for rollback:

   ```sh
   docker inspect --format '{{.Config.Image}}' "$(docker compose ps -q server)" > previous-server-image.txt
   ```

2. Create and inspect a database backup. Choose a new filename if this one exists:

   ```sh
   umask 077
   docker compose exec -T postgres pg_dump -U arenabuddy -d arenabuddy -Fc > before-match-ownership.dump
   docker compose exec -T postgres pg_restore --list < before-match-ownership.dump > before-match-ownership.contents
   ```

   Restore this dump into a separate test database and verify the release there.
   A successful archive listing alone does not verify a restore. Do not restore
   over the live database.

3. Inspect ownership counts and export legacy rows and accounts:

   ```sh
   docker compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U arenabuddy -d arenabuddy -c 'SELECT user_id, count(*) FROM match GROUP BY user_id ORDER BY user_id;'
   docker compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U arenabuddy -d arenabuddy -c 'COPY (SELECT id, controller_player_name, opponent_player_name, created_at FROM match WHERE user_id IS NULL ORDER BY created_at, id) TO STDOUT WITH CSV HEADER' > unowned-matches.csv
   docker compose exec -T postgres psql -X -v ON_ERROR_STOP=1 -U arenabuddy -d arenabuddy -c 'COPY (SELECT id, discord_id, username FROM app_user ORDER BY id) TO STDOUT WITH CSV HEADER' > match-accounts.csv
   ```

## Assign verified legacy owners

If no unowned matches need assignment, skip this section. Otherwise, confirm
ownership using trusted historical records or the account holder's local data.
Create `match-owners.csv` with these columns and one reviewed assignment per row:

```csv
match_id,user_id
```

Use existing UUIDs from the exports. Do not map all rows to the first account or
infer accounts solely from player names. Preserve the reviewed CSV with the backup
as the record of the assignments.

Copy the CSV and the script from this release into the PostgreSQL container:

```sh
docker compose cp match-owners.csv postgres:/tmp/match-owners.csv
docker compose cp ops/assign-match-owners.sql postgres:/tmp/assign-match-owners.sql
```

Preview the assignments without persisting changes:

```sh
docker compose exec -T -w /tmp postgres psql -X -U arenabuddy -d arenabuddy -f assign-match-owners.sql
```

The script rejects duplicate matches, missing IDs, unknown users, and changes to
existing owners. It locks match writes during validation and assignment and
rolls back on error. Keep the mapping small enough for a brief maintenance window.
After reviewing the preview, apply it:

```sh
docker compose exec -T -w /tmp postgres psql -X -v apply=true -U arenabuddy -d arenabuddy -f assign-match-owners.sql
```

Rerunning the same mapping is safe: rows already assigned to the specified owner
remain unchanged. Remove the temporary container copies after applying it:

```sh
docker compose exec -T postgres rm /tmp/match-owners.csv /tmp/assign-match-owners.sql
```

## Deploy and verify

Deploy the server image through the existing workflow after the review. If you
disabled the workflow, re-enable it and dispatch the release. Keep the database
volume, account records, and `JWT_SECRET` unchanged.

For a manual server-only deployment, set `SERVER_IMAGE` to the built release's
immutable image tag and run:

```sh
docker compose pull server
docker compose up -d --no-deps server
docker compose logs --since 5m server
```

On the restored test database, verify these cases before deploying. After
deployment, repeat normal upload and download checks with an existing client:

- An existing owner's retry succeeds without duplicating records.
- A new match uploads successfully and appears only in its owner's list.
- Another account cannot read, classify, or delete that match.
- Another account's upload of that ID returns `PERMISSION_DENIED`; the original
  match, decks, results, mulligans, opponent cards, and event logs remain unchanged.
- Uploading an unowned legacy ID fails until an operator assigns it.
- Unauthenticated requests return `UNAUTHENTICATED`.

Check ownership counts against the pre-deployment export, allowing for new matches
and the reviewed assignments. This release does not repair historical overwrites.
If you find evidence of those, recover affected records from a trusted backup or
the owner's local database.

Older clients do not automatically requeue failed uploads. After an assignment,
use the client's existing manual match upload action, or reprocess the source log
if that client version lacks the action. Restart clients that started while the
server was unavailable so their ingestion upload connection is initialized again.

## Roll back

This change adds no schema migration, so an application rollback does not require
restoring the database. Restore the recorded server image through your deployment
process, or run:

```sh
export SERVER_IMAGE="$(cat previous-server-image.txt)"
docker compose up -d --no-deps server
```

Retain the reviewed owner assignments. Restoring the full backup would discard
matches written after the backup. If an assignment was incorrect, stop server
writes and correct only the reviewed rows in a separate transaction.

Warning: The previous server image reintroduces the ownership vulnerability.
If possible, keep uploads unavailable while preparing a forward fix instead of
running the previous image against a shared database.

## Run regression tests

Use a disposable PostgreSQL instance. SQLx creates and migrates isolated test
databases, so the database role must have `CREATEDB`. Never point these tests at
production. Set `DATABASE_URL` to that test instance, then run:

```sh
SQLX_OFFLINE=true cargo test -p arenabuddy_data -p arenabuddy_server
```
