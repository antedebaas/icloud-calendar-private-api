#!/bin/bash

# Start the iCloud Calendar REST API Server

echo "🚀 Starting iCloud Calendar API Server..."
echo ""

# Check if config.toml exists
if [ ! -f "config.toml" ]; then
    echo "❌ config.toml not found!"
    echo "   Please create it from config.example.toml:"
    echo "   cp config.example.toml config.toml"
    echo "   Then edit config.toml with your iCloud credentials"
    exit 1
fi

# Build if needed
if [ ! -f "target/release/icloud-calendar-private-api" ]; then
    echo "📦 Building server (first time only)..."
    cargo build --release --bin icloud-calendar-private-api
    echo ""
fi

# Start the server
cargo run --bin icloud-calendar-private-api --release
