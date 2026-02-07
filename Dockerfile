# Multi-stage Dockerfile for HyperBox Production Deployment
# Optimized for security, size, and performance

# ============================================================================
# Stage 1: Builder
# ============================================================================
FROM rust:latest as builder

WORKDIR /build

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY app ./app
COPY tests ./tests

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    build-essential \
    && rm -rf /var/lib/apt/lists/*

# Build optimized release binary
RUN cargo build --release --bin hyperboxd --bin hb

# ============================================================================
# Stage 2: Runtime
# ============================================================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    libssl-dev \
    libc6 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user for security
RUN useradd -m -u 1000 hyperbox && \
    mkdir -p /var/lib/hyperbox && \
    chown -R hyperbox:hyperbox /var/lib/hyperbox

# Copy binaries from builder
COPY --from=builder /build/target/release/hyperboxd /usr/local/bin/
COPY --from=builder /build/target/release/hb /usr/local/bin/

# Make binaries executable
RUN chmod +x /usr/local/bin/hyperboxd /usr/local/bin/hb

# Set working directory
WORKDIR /home/hyperbox

# Switch to non-root user
USER hyperbox

# Health check
HEALTHCHECK --interval=30s --timeout=3s --start-period=5s --retries=3 \
    CMD hb health || exit 1

# Expose daemon port
EXPOSE 9999

# Default command: run daemon
ENTRYPOINT ["/usr/local/bin/hyperboxd"]

# Labels for metadata
LABEL maintainer="HyperBox Development Team"
LABEL version="0.1.0"
LABEL description="HyperBox: Container Memory Optimization and Performance Acceleration"
