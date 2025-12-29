#!/bin/bash

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(dirname "$SCRIPT_DIR")"
FRONTEND_DIR="$PROJECT_ROOT/frontend"
OUTPUT_DIR="$PROJECT_ROOT/crates/orchestrate-web/static"

echo "Building frontend..."

# Install dependencies if needed
if [ ! -d "$FRONTEND_DIR/node_modules" ]; then
    echo "Installing dependencies..."
    cd "$FRONTEND_DIR"
    npm install
fi

# Build frontend
cd "$FRONTEND_DIR"
npm run build

echo "Frontend built successfully to $OUTPUT_DIR"
