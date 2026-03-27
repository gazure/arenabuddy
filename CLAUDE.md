# CLAUDE.md

## Commands

### Workspace-wide

```bash
cargo +nightly fmt --all # format all code
cargo +nightly fmt --all -- --check # check formatting (CI)
cargo clippy --profile ci --all --all-targets --all-features --examples --tests   # lint (CI)
cargo check --profile ci --workspace --all-targets --all-features # type-check (CI)
cargo test -p <crate> # test a specific crate
```

New code must pass clippy and fmt before committing.

### Workspace lints

maintained in Cargo.toml

### Rust edition

maintained in Cargo.toml
