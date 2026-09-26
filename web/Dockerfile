FROM rust:1.96-bookworm AS builder
RUN rustup target add wasm32-unknown-unknown
RUN cargo install dioxus-cli@0.8.0-alpha.1 --locked
WORKDIR /app
COPY . .
RUN cd web && dx build --release --platform web --debug-symbols false

FROM caddy:2-alpine AS runtime
COPY --from=builder /app/target/dx/arenabuddy_web/release/web/public /srv
COPY web/Caddyfile /etc/caddy/Caddyfile
