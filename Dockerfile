# Backend Dockerfile for game-server
FROM rust:1.89-slim AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
    curl \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy Cargo files
COPY Cargo.toml Cargo.lock ./

# Copy all workspace members
COPY game-core game-core
COPY game-types game-types  
COPY game-persistence game-persistence
COPY game-server game-server
COPY migration migration

# Copy and run word generation script
COPY scripts/download_and_split_words.sh scripts/
RUN chmod +x scripts/download_and_split_words.sh && \
    ./scripts/download_and_split_words.sh word_lists

# Build the application
RUN cargo build --release --bin game-server

# Runtime stage - use debian slim for SQLite compatibility
FROM debian:bookworm-slim

# Install minimal runtime dependencies
RUN apt-get update && apt-get install -y \
    ca-certificates \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/game-server /app/game-server

# Copy word lists from build stage
COPY --from=builder /app/word_lists ./word_lists

# Set environment variables
ENV RUST_LOG=info
ENV WORDS_DIRECTORY=/app/word_lists

EXPOSE 8080

CMD ["./game-server"]
