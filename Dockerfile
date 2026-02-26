# Build stage
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/vex

# Install required dependencies for building
RUN apt-get update && \
    apt-get install -y pkg-config libssl-dev build-essential && \
    rm -rf /var/lib/apt/lists/*

# Copy the entire workspace
COPY . .

# Build the API server in release mode
RUN cargo build --release -p vex-api

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (CA certs are essential for LLM API calls)
RUN apt-get update && \
    apt-get install -y ca-certificates libssl3 curl && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binary from the builder stage
COPY --from=builder /usr/src/vex/target/release/vex-api /usr/local/bin/

# Create a data directory for the persistent SQLite volume
RUN mkdir -p /data && chown -R 1000:1000 /data

# Set secure defaults
ENV VEX_PORT=8080
ENV VEX_ENV=production
ENV DATABASE_URL="sqlite:///data/vex.db?mode=rwc"

# Run as an unprivileged user for security
USER 1000:1000

# Expose the port
EXPOSE 8080

# Healthcheck to aid orchestration
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:8080/health || exit 1

# Start the VEX API
CMD ["vex-api"]
