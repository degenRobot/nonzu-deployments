# Multi-stage Dockerfile for binance-oracle
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
RUN cargo build --release --bin binance-oracle

# Strip debug symbols to reduce binary size
RUN strip target/release/binance-oracle

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
COPY --from=builder /app/target/release/binance-oracle /usr/local/bin/binance-oracle

# Create directory for any runtime data
RUN mkdir -p /app/data && chown oracle:oracle /app/data

# Switch to non-root user
USER oracle

# Set working directory
WORKDIR /app

# No ports exposed - this is a background service

# Set default environment variables
ENV RUST_LOG=info,nonzu_sdk=warn,binance_oracle=info \
    RUST_BACKTRACE=1 \
    NETWORK=testnet

# Run the oracle
ENTRYPOINT ["/usr/local/bin/binance-oracle"]