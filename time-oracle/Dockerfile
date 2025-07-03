# Multi-stage Dockerfile for time-oracle
# Optimized for production deployment on Fly.io

# Stage 1: Build environment
FROM rust:latest AS builder

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Create app directory
WORKDIR /app

# Copy manifest files first for better caching
COPY Cargo.toml Cargo.lock ./

# Copy vendored SDK
COPY vendor ./vendor

# Copy source code
COPY src ./src

# Build in release mode with optimizations
RUN cargo build --release --bin time-oracle

# Strip debug symbols to reduce binary size
RUN strip target/release/time-oracle

# Stage 2: Runtime environment
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1001 oracle

# Copy binary from builder
COPY --from=builder /app/target/release/time-oracle /usr/local/bin/time-oracle

# Create directory for any runtime data
RUN mkdir -p /app/data && chown oracle:oracle /app/data

# Switch to non-root user
USER oracle

# Set working directory
WORKDIR /app

# No ports exposed - this is a background service

# Set default environment variables
ENV RUST_LOG=info,noboru_sdk=debug,time_oracle=debug \
    RUST_BACKTRACE=1 \
    UPDATE_INTERVAL_MS=100 \
    NETWORK=testnet 

# Run the oracle
ENTRYPOINT ["/usr/local/bin/time-oracle"]