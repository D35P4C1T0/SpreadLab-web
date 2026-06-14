FROM rust:bookworm AS builder

WORKDIR /app
COPY . .
RUN cargo build --release -p spreadlab-web

FROM debian:bookworm-slim

WORKDIR /app
RUN apt-get update \
    && apt-get install -y --no-install-recommends ca-certificates \
    && rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/spreadlab-web /usr/local/bin/spreadlab-web
COPY --from=builder /app/crates/spreadlab-web/assets crates/spreadlab-web/assets

EXPOSE 3000

CMD ["spreadlab-web", "serve", "--host", "0.0.0.0", "--port", "3000"]
