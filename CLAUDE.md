# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Word Arena is a real-time multiplayer Wordle-style game built with a React TypeScript frontend and Rust backend. Players collaborate to solve word puzzles while competing for points in rounds-based matches.

## Architecture

**Monorepo Structure:**
- `frontend/` - React TypeScript SPA with Vite, Tailwind CSS, Zustand state management
- `game-core/` - Pure game logic (scoring, word validation, game state)
- `game-types/` - Shared types with automatic TypeScript generation via ts-rs
- `game-persistence/` - Database layer using SeaORM with SQLite
- `game-server/` - WebSocket server using Warp for real-time communication
- `migration/` - Database migration management

## Common Development Commands

**Development Workflow:**
```bash
npm run dev                    # Start both frontend (port 3000) and backend (port 8080)
npm run dev:frontend          # Start only frontend
npm run dev:backend           # Start only backend (cargo run -p game-server)
```

**Building:**
```bash
npm run build                 # Build both frontend and backend
npm run build:frontend        # Build frontend (tsc && vite build)
npm run build:backend         # Build backend (cargo build --release)
```

**Testing:**
```bash
npm run test                  # Run all tests (frontend + backend)
npm run test:frontend         # Frontend tests (vitest)
npm run test:backend          # Backend tests (cargo test --workspace)
```

**Type Generation:**
```bash
npm run types:generate        # Generate TypeScript types from Rust (cargo test -p game-types)
```

**Database Management:**
```bash
npm run db:migrate           # Run database migrations
npm run db:reset             # Reset database (rm word_arena.db && migrate)
```

**Linting:**
```bash
cd frontend && npm run lint   # ESLint for frontend
cargo clippy --workspace     # Clippy for backend
```

## Type Safety System

The project uses ts-rs to automatically generate TypeScript types from Rust structs and enums. When modifying types in `game-types/src/`, run `npm run types:generate` to update the corresponding TypeScript definitions in `frontend/src/types/generated/`.

## Key Architectural Patterns

**Backend Crates:**
- `game-core`: Pure business logic, no I/O dependencies
- `game-types`: Shared data structures with ts-rs derives for TypeScript export
- `game-persistence`: Repository pattern with SeaORM entities
- `game-server`: WebSocket handlers, matchmaking, connection management

**Frontend Structure:**
- React components organized by feature (auth/, game/, lobby/)
- Zustand stores for state management
- WebSocket service for real-time communication
- Generated types imported from backend

**Real-time Communication:**
All client-server communication happens via WebSockets using message types defined in `game-types/src/messages.rs`. Rate limiting and connection management are handled in `game-server/src/websocket/`.

## Game Logic

**Core Flow:**
1. Players join matchmaking queue (2-16 players)
2. Rounds consist of simultaneous guessing followed by winner's individual guess
3. Points awarded for new letter discoveries (1pt orange, 2pt blue, 5pt word completion)
4. Games continue until point threshold reached

**Scoring System:**
Implemented in `game-core/src/scoring.rs` with deterministic point calculation based on new information revealed per guess.

## Database Development

Uses SeaORM with SQLite for development, easily upgradeable to PostgreSQL for production. Entity definitions are in `game-persistence/src/entities/` and migrations in `migration/src/`.

## Development Environment

The backend server runs on port 8080 with WebSocket endpoint at `/ws`. Frontend development server runs on port 3000 with proxy configuration for API calls.