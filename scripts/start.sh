#!/bin/bash

# Start script for arb-bot

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

# Check if config exists
if [ ! -f config.toml ]; then
    echo "Error: config.toml not found. Please create it from config.example.toml"
    exit 1
fi

# Start the bot
echo "Starting arb-bot..."
cargo run --release







