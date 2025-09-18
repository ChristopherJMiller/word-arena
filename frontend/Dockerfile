# Frontend Dockerfile
FROM node:20-alpine AS builder

# Accept build arguments for environment configuration
ARG VITE_AZURE_CLIENT_ID=dev-client-id
ARG VITE_AZURE_TENANT_ID=common
ARG VITE_AUTH_DEV_MODE=false
ARG VITE_WS_URL=ws://localhost:8080/ws

# Set environment variables from build args
ENV VITE_AZURE_CLIENT_ID=${VITE_AZURE_CLIENT_ID}
ENV VITE_AZURE_TENANT_ID=${VITE_AZURE_TENANT_ID}
ENV VITE_AUTH_DEV_MODE=${VITE_AUTH_DEV_MODE}
ENV VITE_WS_URL=${VITE_WS_URL}

# Install Rust for type generation
RUN apk add --no-cache curl build-base
RUN curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
ENV PATH="/root/.cargo/bin:${PATH}"

WORKDIR /app

# Copy root package files (workspace setup)
COPY package*.json ./
COPY frontend/package*.json ./frontend/

# Install dependencies
RUN npm ci

# Copy Cargo files for type generation
COPY Cargo.toml Cargo.lock ./
COPY game-types ./game-types
COPY game-core ./game-core
COPY game-persistence ./game-persistence
COPY game-server ./game-server
COPY migration ./migration

# Copy frontend source code
COPY frontend ./frontend

# Generate types
RUN npm run types:generate

# Build the frontend
RUN npm run build:frontend

# Production stage - serve with nginx
FROM nginx:alpine

# Copy built assets
COPY --from=builder /app/frontend/dist /usr/share/nginx/html

# Copy nginx configuration
COPY frontend/nginx.conf /etc/nginx/nginx.conf

EXPOSE 8080

CMD ["nginx", "-g", "daemon off;"]