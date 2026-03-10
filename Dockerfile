# Build stage
FROM rust:slim-bookworm AS builder

WORKDIR /usr/src/vex

# Install required dependencies for building
# Mask udevadm and divert tpm-udev postinst to prevent crashes on read-only /sys platforms like Railway
RUN apt-get update && \
    # 1. Mask udevadm in all common locations
    ln -sf /bin/true /usr/local/bin/udevadm && \
    ln -sf /bin/true /usr/bin/udevadm && \
    ln -sf /bin/true /bin/udevadm && \
    ln -sf /bin/true /sbin/udevadm && \
    # 2. Divert tpm-udev script before it's even installed
    mkdir -p /var/lib/dpkg/info/ && \
    dpkg-divert --local --rename --add /var/lib/dpkg/info/tpm-udev.postinst && \
    ln -sf /bin/true /var/lib/dpkg/info/tpm-udev.postinst && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    pkg-config libssl-dev build-essential curl libtss2-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy pre-compiled Go toolchain directly from the official image to avoid curl/SSL issues
COPY --from=golang:1.24.0-bookworm /usr/local/go /usr/local/go
ENV PATH=$PATH:/usr/local/go/bin

# Copy the entire workspace
COPY . .

# Build the API server and Attest Rust core in release mode
# Use --jobs 2 to reduce memory consumption and prevent OOM on constrained builders (like Railway)
RUN cargo build --release --jobs 2 -p vex-server -p attest-rs

# Create a debug symlink just in case any CGO paths are hardcoded to "debug"
RUN ln -s /usr/src/vex/target/release /usr/src/vex/target/debug

# Build the Attest CLI
ENV CGO_LDFLAGS="-L/usr/src/vex/target/release"
RUN cd attest && go build -v -o ../attest-bin ./cmd/attest

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies (CA certs are essential for LLM API calls)
# Mask udevadm and divert tpm-udev postinst to prevent crashes on Railway
RUN apt-get update && \
    # 1. Mask udevadm in all common locations
    ln -sf /bin/true /usr/local/bin/udevadm && \
    ln -sf /bin/true /usr/bin/udevadm && \
    ln -sf /bin/true /bin/udevadm && \
    ln -sf /bin/true /sbin/udevadm && \
    # 2. Divert tpm-udev script before it's even installed
    mkdir -p /var/lib/dpkg/info/ && \
    dpkg-divert --local --rename --add /var/lib/dpkg/info/tpm-udev.postinst && \
    ln -sf /bin/true /var/lib/dpkg/info/tpm-udev.postinst && \
    DEBIAN_FRONTEND=noninteractive apt-get install -y --no-install-recommends \
    ca-certificates libssl3 curl libtss2-esys-3.0.2-0 libtss2-tctildr0 && \
    rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the compiled binaries from the builder stage
COPY --from=builder /usr/src/vex/target/release/vex-server /usr/local/bin/
COPY --from=builder /usr/src/vex/attest-bin /usr/local/bin/attest

# Create a data directory for the persistent SQLite volume
RUN mkdir -p /data && chown -R 1000:1000 /data

# Set secure defaults
ENV VEX_PORT=8080
ENV VEX_ENV=railway
ENV DATABASE_URL="sqlite:///data/vex.db?mode=rwc"

# Run as an unprivileged user for security
USER 1000:1000

# Expose the port
EXPOSE 8080

# Healthcheck to start correctly
HEALTHCHECK --interval=30s --timeout=10s --start-period=5s --retries=3 \
  CMD curl -f http://localhost:${VEX_PORT:-8080}/health || exit 1

# Start the VEX Server
CMD ["vex-server"]
