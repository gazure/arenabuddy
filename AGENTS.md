# AGENTS.md

## Cursor Cloud specific instructions

### Overview

ArenaBuddy is an MTGA companion app. The workspace has 7 crates: `core`, `data`, `cli`, `arenabuddy` (Dioxus desktop), `server` (gRPC), `web` (Dioxus WASM), and `metagame`. See `README.md` for project structure.

### Build and check commands

All standard commands are in `CLAUDE.md`. Key point: always set `SQLX_OFFLINE=true` when running `cargo check`, `cargo clippy`, or `cargo test` — the `.sqlx/` directory has cached query metadata so no live PostgreSQL is needed for compilation.

```bash
SQLX_OFFLINE=true cargo check --profile ci --workspace --all-targets --all-features
SQLX_OFFLINE=true cargo clippy --profile ci --all --all-targets --all-features --examples --tests
cargo +nightly fmt --all -- --check
SQLX_OFFLINE=true cargo test -p <crate>
```

### System dependencies (pre-installed in snapshot)

- `protobuf-compiler` — required by `core/build.rs` to compile `.proto` files
- GTK/WebKit dev libraries — required for the Dioxus desktop app (`arenabuddy` crate): `libwebkit2gtk-4.1-dev`, `libgtk-3-dev`, `libayatana-appindicator3-dev`, `libssl-dev`, `libxdo-dev`, `libsoup-3.0-dev`, `librsvg2-dev`

### Gotchas

- The `data` crate uses `sqlx.toml` with `database-url-var = "ARENABUDDY_DATABASE_URL"` (not the standard `DATABASE_URL`). For offline mode set `SQLX_OFFLINE=true`.
- Rust edition 2024 requires `rust-version = "1.94"`. The snapshot has stable 1.94+ and nightly installed.
- The desktop app (`arenabuddy` crate) and web app (`web` crate) use Dioxus 0.7.3. Building them with `dx serve` requires the Dioxus CLI, but `cargo check`/`clippy`/`test` work without it.
- The server requires PostgreSQL, Discord OAuth credentials, and a JWT secret to run. For compile/test, `SQLX_OFFLINE=true` is sufficient.
- Integration tests for `arenabuddy_data` may attempt to start an embedded PostgreSQL; this is handled by the `postgresql_embedded` crate with the `bundled` feature.
