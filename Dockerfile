# Backend Dockerfile for game-server
FROM rust:1.89-slim AS builder

# Install dependencies
RUN apt-get update && apt-get install -y \
    pkg-config \
    libssl-dev \
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

# Build the application
RUN cargo build --release --bin game-server

# Runtime stage - use distroless for security
FROM gcr.io/distroless/cc-debian12

WORKDIR /app

# Copy the binary
COPY --from=builder /app/target/release/game-server /app/game-server

# Copy word lists if they exist
COPY word_lists ./word_lists

# Set environment variables
ENV DATABASE_URL=sqlite:///app/data/word_arena.db
ENV RUST_LOG=info

EXPOSE 8080

CMD ["./game-server"]