# Word Arena

A real-time multiplayer Wordle-style game built with React and Rust. Players collaborate to solve word puzzles while competing for points in rounds-based matches.

## Game Overview

Word Arena is a competitive twist on Wordle where 2-16 players simultaneously guess words in real-time. The first player to make progress advances to an individual guess phase, with points awarded for discovering new letters and solving words.

## 🏗️ Architecture

This is a monorepo containing:

- **Frontend**: React 18 + TypeScript + Vite + Tailwind CSS
- **Backend**: Rust + Warp (WebSocket server) + SeaORM (SQLite)
- **Shared Types**: Automatic TypeScript generation from Rust using ts-rs
- **Database**: SQLite for development, easily upgradeable to PostgreSQL

```
word-arena/
├── frontend/          # React TypeScript SPA
├── game-core/         # Pure game logic (scoring, validation)
├── game-types/        # Shared types with ts-rs exports
├── game-persistence/  # Database layer with SeaORM
├── game-server/       # WebSocket server and HTTP endpoints
└── migration/         # Database migrations
```

## 🚀 Quick Start

### Prerequisites

- **Rust** (latest stable) - [Install Rust](https://rustup.rs/)
- **Node.js** (16+) - [Install Node.js](https://nodejs.org/)
- **SeaORM CLI** - `cargo install sea-orm-cli`

### 1. Clone and Install Dependencies

```bash
git clone <repository-url>
cd word-arena

# Install frontend dependencies
npm install

# Install Rust dependencies (automatically handled by Cargo)
```

### 2. Database Setup

```bash
# Run database migrations
npm run db:migrate

# Or reset database if needed
npm run db:reset
```

### 3. Generate TypeScript Types

```bash
# Generate TypeScript types from Rust structs
npm run types:generate
```

### 4. Start Development Servers

```bash
# Start both frontend and backend
npm run dev

# Or start individually:
npm run dev:frontend  # Frontend on http://localhost:3000
npm run dev:backend   # Backend on ws://localhost:8080
```

### 5. Access the Application

- **Frontend**: http://localhost:3000
- **Backend Health**: http://localhost:8080/health
- **Leaderboard API**: http://localhost:8080/leaderboard

## 🛠️ Development

### Available Commands

```bash
# Development
npm run dev                    # Start both frontend and backend
npm run dev:frontend          # Start only frontend (port 3000)
npm run dev:backend           # Start only backend (port 8080)

# Building
npm run build                 # Build both frontend and backend
npm run build:frontend        # Build frontend (tsc && vite build)
npm run build:backend         # Build backend (cargo build --release)

# Testing
npm run test                  # Run all tests (frontend + backend)
npm run test:frontend         # Frontend tests (vitest)
npm run test:backend          # Backend tests (cargo test --workspace)

# Type Generation
npm run types:generate        # Generate TypeScript types from Rust

# Database
npm run db:migrate           # Run database migrations
npm run db:reset             # Reset database (rm word_arena.db && migrate)

# Linting
cd frontend && npm run lint   # ESLint for frontend
cargo clippy --workspace     # Clippy for backend
```

### Development Authentication

The project includes a development mode for easy testing:

1. Set environment variable: `AUTH_DEV_MODE=true`
2. Use the dev login form in the frontend with preset users or create custom ones
3. Authentication tokens are simple strings like `user1:alice@example.com:Alice`

### Database Development

```bash
# Create new migration
cargo run -p migration -- generate create_new_table

# Generate entities from existing database
./generate_entities.sh

# Reset database for fresh start
npm run db:reset
```

## 🏛️ Project Structure

### Frontend (`frontend/`)

- **Components**: Organized by feature (auth/, game/, lobby/)
- **State Management**: Zustand stores
- **Styling**: Tailwind CSS with responsive design
- **Types**: Auto-generated from Rust backend

### Backend Crates

- **`game-core`**: Pure business logic, no I/O dependencies
- **`game-types`**: Shared data structures with ts-rs derives
- **`game-persistence`**: Repository pattern with SeaORM entities
- **`game-server`**: WebSocket handlers, HTTP endpoints, connection management
- **`migration`**: Database schema management

### Key Files

- **`package.json`**: Root workspace with unified commands
- **`Cargo.toml`**: Rust workspace configuration
- **`CLAUDE.md`**: Development guidance and patterns
- **`generate_entities.sh`**: Automated SeaORM entity generation

## 🎯 Game Rules

1. **Matchmaking**: 2-16 players join a queue
2. **Round Phase**: All players simultaneously guess words
3. **Winner Selection**: Player with most correct letters advances
4. **Individual Phase**: Winner gets exclusive guess at the word
5. **Scoring**:
   - 1 point for each new orange letter (present but wrong position)
   - 2 points for each new green letter (correct position)
   - 5 points for solving the word
6. **Victory**: First player to reach point threshold wins

## 🔧 Configuration

### Environment Variables

```bash
# Backend
DATABASE_URL=sqlite://word_arena.db    # Database connection string
AUTH_DEV_MODE=true                     # Enable development authentication
AZURE_TENANT_ID=your-tenant-id         # Production Azure AD tenant
AZURE_CLIENT_ID=your-client-id         # Production Azure AD client

# Frontend
VITE_AUTH_DEV_MODE=true               # Enable dev mode in frontend
```

### Production Deployment

**Frontend**: Build and deploy to CDN (Vercel/Netlify/CloudFlare)

```bash
npm run build:frontend
# Deploy frontend/dist/ to your CDN
```

**Backend**: Single binary deployment

```bash
npm run build:backend
# Deploy target/release/game-server binary
```

## 🧪 Testing

The project includes comprehensive testing:

- **Frontend**: Component tests with Vitest and React Testing Library
- **Backend**: Unit tests for pure logic, integration tests with in-memory SQLite
- **Persistence**: Full repository testing with automated migrations
- **HTTP APIs**: Route testing for leaderboard and user stats endpoints

```bash
# Run specific test suites
cargo test -p game-core          # Pure game logic tests
cargo test -p game-persistence   # Database operation tests
cargo test -p game-server        # Integration and HTTP tests
cd frontend && npm run test      # Frontend component tests
```

## 📊 API Reference

### WebSocket Messages

- **Client → Server**: `JoinQueue`, `LeaveQueue`, `SubmitGuess`, `Authenticate`
- **Server → Client**: `MatchFound`, `GameStateUpdate`, `RoundResult`, `GameOver`

### HTTP Endpoints

- **GET** `/health` - Health check
- **GET** `/leaderboard?limit=N` - Global leaderboard (max 100)
- **GET** `/user/{id}/stats` - User statistics and rank (authenticated)
- **GET** `/game/{id}/state` - Safe game state for reconnection

## 🤝 Contributing

1. Follow existing code patterns and architecture
2. Run tests before submitting: `npm run test`
3. Use provided linting: `cargo clippy` and `npm run lint`
4. Generate types after modifying `game-types`: `npm run types:generate`

**Database issues**: Reset database

```bash
npm run db:reset
```

**Type mismatches**: Regenerate TypeScript types

```bash
npm run types:generate
```

**Build failures**: Clean and rebuild

```bash
cargo clean
rm -rf frontend/dist frontend/node_modules
npm install
npm run build
```

### Development Tips

- Use `AUTH_DEV_MODE=true` for easier local testing
- Check browser console and server logs for WebSocket issues
- Database schema changes require running migrations and regenerating entities
- Frontend hot reload works with Vite, backend requires restart after changes
