#!/bin/bash

# Update script for arb-bot

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

cd "$PROJECT_DIR"

echo "Updating arb-bot..."

# Pull latest changes
if [ -d .git ]; then
    git pull
fi

# Update dependencies and rebuild
cargo update
cargo build --release

echo "Update complete!"







