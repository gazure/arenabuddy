FROM rust:1.94-bookworm AS builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli@0.7.3 --locked
WORKDIR /app
COPY . .
RUN cd arenabuddy/web && dx build --release --platform web

FROM caddy:2-alpine AS runtime
COPY --from=builder /app/target/dx/arenabuddy_web/release/web/public /srv
COPY arenabuddy/web/Caddyfile /etc/caddy/Caddyfile
