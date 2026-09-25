# Database lifecycle and repository access

`Database` owns connections, embedded PostgreSQL startup, migrations, and shutdown.
`MatchDB` holds a pool and the card lookup database. Repository traits expose
application queries and writes; they do not expose initialization. All repository
and writer traits use `async_trait`.

## Application startup

Connect and migrate before passing a repository to application services:

```rust,no_run
use arenabuddy_core::cards::CardsDatabase;
use arenabuddy_data::{Database, Result};

async fn run(url: &str) -> Result<()> {
    let database = Database::connect(url).await?;
    database.migrate().await?;
    let repository = database.repository(CardsDatabase::default());

    // Pass repository clones to application services here.

    drop(repository);
    database.close().await?;
    Ok(())
}
```

`Database::connect` does not apply migrations. Existing server, CLI, scraper, and
desktop entry points explicitly call `migrate` before starting their work, which
preserves their previous startup behavior. Migration failures stop startup.

To use a pool configured elsewhere, call `MatchDB::from_pool(pool, cards)`. This
constructor does not connect or change the schema. Use `Database::from_pool` when
you also need migration or shutdown operations. A deployment process can run
migrations separately and construct repositories with a runtime database role;
this refactor does not introduce that deployment policy or change database grants.

## Embedded PostgreSQL

The desktop uses `Database::start_embedded` when `ARENABUDDY_DATABASE_URL` is unset.
Keep the returned owner alive for the full application lifetime. Repository clones
share the pool but do not own the PostgreSQL process.

The installation, data directory, database name, and password location retain the
values used by existing installations. `start_embedded_at` accepts a separate
application directory for tests or custom hosts. Neither startup method migrates
the schema; call `migrate` explicitly.

Startup takes an exclusive lock on `arenabuddy.lock`. Another application using
the same directory gets an error before PostgreSQL setup begins. A preexisting
`data/postmaster.pid` also stops startup. Close the previous application first.
After an abnormal exit, inspect whether the database process is still running
before repairing a stale PID file. Startup no longer stops an existing process
or deletes its PID file automatically.

The desktop handles normal event-loop shutdown by stopping its background workers,
closing the pool, and stopping its owned database. `Database::close` closes all
clones of its pool and waits for checked-out connections to return. It never stops
an external PostgreSQL server. Dropping the owner provides embedded-process cleanup
as a fallback; abrupt process termination cannot guarantee cleanup.

## Existing clients and droplet data

This refactor adds no schema migration or protocol change. Existing server records,
client data directories, migration history, and upload jobs remain in place. The
server uses its existing `DATABASE_URL`, and the desktop retains its existing
`ARENABUDDY_DATABASE_URL` override. No data export or reimport is required.

For Rust callers, replace `MatchDB::new(...).await` and repository `init()` calls
with explicit `Database` startup, `migrate()`, and `repository(cards)` calls. These
are source API changes; older deployed clients do not need to upgrade in lockstep.

## Validation

The database tests check explicit initialization, migration error propagation,
repeated migrations against existing records, pool ownership, and refusal to
modify an existing PID file. Run the bundled PostgreSQL restart test explicitly:

```sh
SQLX_OFFLINE=true cargo test -p arenabuddy_data \
  embedded_restart_preserves_data_and_repository_drops_do_not_stop_postgres -- --ignored
```

This test starts PostgreSQL in a temporary directory, saves a record, closes the
database, and verifies the record after restart. It requires an environment where
PostgreSQL can run as a non-root user. Other SQLx database tests use an isolated
test database on the server specified by `DATABASE_URL`.
