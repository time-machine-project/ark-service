# The builder's Debian release must match the runtime image, or the binary needs a newer glibc
FROM rust:1.90-slim-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./

# Builds the dependencies in their own layer, so source changes don't rebuild them
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release --locked && \
    rm -rf src

COPY src ./src

# The source must look newer than the dummy main.rs for cargo to rebuild it
RUN touch src/main.rs && \
    cargo build --release --locked

FROM debian:bookworm-slim

WORKDIR /app

RUN apt-get update && \
    apt-get install -y ca-certificates curl && \
    rm -rf /var/lib/apt/lists/*

COPY --from=builder /app/target/release/ark-service /usr/local/bin/ark-service

RUN useradd -m -u 1000 arkuser && \
    chown -R arkuser:arkuser /app

USER arkuser

EXPOSE 3000

HEALTHCHECK --interval=30s --timeout=3s --start-period=10s --retries=3 \
    CMD curl -fsS "http://localhost:${PORT:-3000}/ark:$(printf %s "$NAAN" | tr '[:upper:]' '[:lower:]')/servicestatus" || exit 1

CMD ["/usr/local/bin/ark-service"]
