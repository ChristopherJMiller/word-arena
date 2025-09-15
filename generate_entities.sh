#!/bin/bash

# Generate SeaORM entities from database schema
# This script runs migrations and then generates entities using SeaORM CLI

set -e  # Exit on any error

DB_FILE="$(pwd)/temp_word_arena.db"
DB_URL="sqlite://$DB_FILE"
ENTITIES_DIR="game-persistence/src/entities"

echo "🚀 Starting SeaORM entity generation..."

# Clean up any existing temporary database
if [ -f "$DB_FILE" ]; then
    echo "🗑️  Removing existing temporary database..."
    rm "$DB_FILE"
fi

# Create fresh database file
echo "📁 Creating temporary database file..."
touch "$DB_FILE"

# Run migrations to create schema
echo "⬆️  Running migrations..."
DATABASE_URL="$DB_URL" cargo run -p migration -- up

# Remove existing entities directory
if [ -d "$ENTITIES_DIR" ]; then
    echo "🗑️  Removing existing entities..."
    rm -rf "$ENTITIES_DIR"
fi

# Generate entities with SeaORM CLI
echo "🔄 Generating entities from database schema..."
DATABASE_URL="$DB_URL" sea-orm-cli generate entity \
    -u "$DB_URL" \
    -o "$ENTITIES_DIR" \
    --with-serde both \
    --date-time-crate chrono \
    --lib

echo "✅ Entity generation complete!"

# Clean up temporary database
echo "🗑️  Cleaning up temporary database..."
rm "$DB_FILE"

echo "🎉 All done! Entities generated in $ENTITIES_DIR"
echo ""
echo "📋 Generated files:"
find "$ENTITIES_DIR" -name "*.rs" | sort