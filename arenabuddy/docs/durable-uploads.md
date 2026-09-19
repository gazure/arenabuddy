# Durable match uploads

The desktop app saves each completed match and an upload snapshot in one local
PostgreSQL transaction. A separate worker uploads snapshots for the signed-in
account. Log ingestion does not wait for uploads, classification, or diagnostic
reporting. If the local transaction fails, neither the match changes nor its
upload job commits.

## Delivery behavior

- The worker checks for due jobs every two seconds, including after startup.
- Temporary failures retry after 5 seconds, then use exponential backoff up to
  320 seconds. Retry timing and the latest error survive restarts.
- Each attempt has a 90-second overall timeout. A job's two-minute lease prevents
  another worker from claiming it during a normal attempt. After a crash, an
  expired lease makes the job available again.
- Delivery is at least once. If the server accepts an upload but the client loses
  the response, the client retries the same snapshot. Match upserts are idempotent;
  the existing Google Sheets append integration can still produce duplicates.
- An unchanged replay does not reset a sent or blocked job. A changed snapshot
  increments the revision. Completion of an older revision cannot clear the newer
  one. Queue updates retain an active lease to keep normal deliveries ordered.
- Permission, payload, unsupported-operation, and precondition errors block the
  job. After correcting the cause, choose **Sync** on the match page to retry.
- Classification runs after a successful upload. It remains best effort and does
  not change upload success. Diagnostic reports use a bounded memory queue and
  can be dropped; they are not part of durable match delivery.

The match details page shows pending, uploaded, and blocked states and refreshes
that status automatically. **Sync** now queues a durable upload instead of waiting
for the server to respond.

## Account handling

The job records the remote account at capture time. Signing out pauses its
delivery; signing into another account does not transfer it. Sign in with the
original account to resume. Account UUIDs are stored without a local `app_user`
foreign key because desktop databases do not contain the server's account table.

Matches captured while signed out are saved with an unassigned job. Sign in and
choose **Sync** for each match you want to upload. An ingestion replay does not
automatically claim an unassigned job. This avoids transferring data to whichever
account happens to sign in next.

## Upgrade existing installations

1. Deploy the ownership enforcement server change first, following the
   [server upgrade guide](../../server/ops/match-ownership-upgrade.md).
2. Back up each desktop database before installing the new client. Keep the
   existing database directory and saved authentication session.
3. Start the new client. Its normal SQLx migration adds
   `match_upload_outbox`; it does not rewrite or delete existing matches. The
   queue keeps one snapshot and delivery record per match.
4. Confirm that a newly captured match shows **Uploaded** when online. Disconnect
   the client from the server, capture a match, restart the client, and reconnect.
   The pending match should upload after its retry delay or lease expires.
5. For older matches that never uploaded, choose **Sync** explicitly. The migration
   does not enqueue the entire database because older rows do not distinguish
   captures from downloads or identify their original remote account.

No gRPC messages or authentication formats change. Existing deployed clients can
continue using the server. Server ingestion and cloud downloads do not create
upload jobs. If you deploy the server from this revision, it applies the shared
additive migration too, but leaves its outbox empty.

## Roll back

Keep the queue table and its SQLx migration record. Rolling back to a binary that
does not include the new migration can fail SQLx's applied-migration validation.
Build a rollback release that retains
`20260919000000_match_upload_outbox.sql` while reverting the worker and UI changes.
Use that compatibility build for either the desktop or server if its database has
already applied this migration.

The previous upload path does not drain durable jobs. Pending jobs remain in the
database and resume when you reinstall the worker. Do not delete the queue to
clear errors: inspect the match status and correct the underlying failure.

## Run tests

Set `DATABASE_URL` to a disposable PostgreSQL instance whose role has `CREATEDB`.
SQLx creates separate test databases; do not use the production server:

```sh
SQLX_OFFLINE=true cargo test -p arenabuddy_data -p arenabuddy_server -p arenabuddy_core -p arenabuddy --lib
```

The database tests cover transaction rollback, reconnects, lease recovery,
concurrent workers, account binding, manual retries, and stale acknowledgments.
Transport tests exercise a transient gRPC failure followed by successful delivery
of the saved payload with the original account's credentials.
