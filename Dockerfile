# Havklo SDK - Multi-stage Docker Build
# Builds the TUI application for container deployment

# ============================================
# Stage 1: Build
# ============================================
FROM rust:1.83-bookworm AS builder

WORKDIR /app

# Install build dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace files
COPY Cargo.toml Cargo.lock ./
COPY crates ./crates
COPY havklo-tui ./havklo-tui

# Build release binary
RUN cargo build --release -p havklo-tui

# ============================================
# Stage 2: Runtime
# ============================================
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    libssl3 \
    && rm -rf /var/lib/apt/lists/*

# Create non-root user
RUN useradd -m -u 1000 havklo

# Copy binary from builder
COPY --from=builder /app/target/release/havklo /usr/local/bin/havklo

# Set ownership
RUN chown havklo:havklo /usr/local/bin/havklo

USER havklo
WORKDIR /home/havklo

# TUI requires interactive terminal
ENV TERM=xterm-256color

ENTRYPOINT ["/usr/local/bin/havklo"]
