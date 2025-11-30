#!/bin/bash

# Stop script for arb-bot

PID=$(pgrep -f "arb-bot")

if [ -z "$PID" ]; then
    echo "arb-bot is not running"
    exit 0
fi

echo "Stopping arb-bot (PID: $PID)..."
kill $PID

# Wait for process to stop
sleep 2

if pgrep -f "arb-bot" > /dev/null; then
    echo "Force killing arb-bot..."
    kill -9 $PID
fi

echo "arb-bot stopped"







