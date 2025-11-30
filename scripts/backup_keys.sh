#!/bin/bash

# Backup keys script

set -e

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(dirname "$SCRIPT_DIR")"

BACKUP_DIR="$PROJECT_DIR/backups"
KEYS_DIR="$PROJECT_DIR/keys"
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
BACKUP_FILE="$BACKUP_DIR/keys_backup_$TIMESTAMP.tar.gz"

mkdir -p "$BACKUP_DIR"

if [ ! -d "$KEYS_DIR" ] || [ -z "$(ls -A $KEYS_DIR 2>/dev/null)" ]; then
    echo "No keys to backup"
    exit 0
fi

echo "Backing up keys to $BACKUP_FILE..."
tar -czf "$BACKUP_FILE" -C "$PROJECT_DIR" keys/

echo "Backup complete: $BACKUP_FILE"

# Keep only last 10 backups
cd "$BACKUP_DIR"
ls -t keys_backup_*.tar.gz | tail -n +11 | xargs -r rm

echo "Old backups cleaned (keeping last 10)"






