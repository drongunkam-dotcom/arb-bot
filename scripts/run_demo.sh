#!/bin/bash

# Demo script for arb-bot

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Running arb-bot in demo mode..."

# Use demo config if available
if [ -f config.demo.toml ]; then
    CONFIG_FILE="config.demo.toml"
else
    CONFIG_FILE="config.example.toml"
fi

echo "Using config: $CONFIG_FILE"
RUST_LOG=debug cargo run --release -- --config "$CONFIG_FILE" --demo






