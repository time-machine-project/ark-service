# Build stage
FROM rust:1.90-slim AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Create a dummy main.rs to cache dependencies
RUN mkdir src && \
    echo "fn main() {}" > src/main.rs && \
    cargo build --release && \
    rm -rf src

# Copy source code
COPY src ./src

# Build the actual application
# Touch main.rs to ensure it's rebuilt
RUN touch src/main.rs && \
    cargo build --release

# Runtime stage
FROM debian:bookworm-slim

WORKDIR /app

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y ca-certificates && \
    rm -rf /var/lib/apt/lists/*

# Copy the binary from builder
COPY --from=builder /app/target/release/ark-service /usr/local/bin/ark-service

# Create a non-root user
RUN useradd -m -u 1000 arkuser && \
    chown -R arkuser:arkuser /app

USER arkuser

# Expose the port
EXPOSE 3000

# Set default environment variables
ENV NAAN="12345" \
    DEFAULT_BLADE_LENGTH="8" \
    MAX_MINT_COUNT="1000" \
    RUST_LOG="info"

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD ["/usr/local/bin/ark-service", "--version"] || exit 1

# Run the application
CMD ["/usr/local/bin/ark-service"]
