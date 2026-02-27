FROM rust:1.93-bookworm AS chef
RUN cargo install cargo-chef@0.1.73
WORKDIR /app

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json --bin arenabuddy_server

FROM chef AS builder
COPY --from=planner /app/recipe.json recipe.json
RUN apt-get update && apt-get install -y protobuf-compiler && rm -rf /var/lib/apt/lists/*
RUN cargo chef cook --release --recipe-path recipe.json --package arenabuddy_server --features otel
COPY . .
RUN cargo build --release --bin arenabuddy_server --features otel

FROM debian:bookworm-slim AS runtime
RUN apt-get update && apt-get install -y ca-certificates tini && rm -rf /var/lib/apt/lists/*
COPY --from=builder /app/target/release/arenabuddy_server /usr/local/bin/arenabuddy_server
ENTRYPOINT ["/usr/bin/tini", "--"]
CMD ["arenabuddy_server"]
