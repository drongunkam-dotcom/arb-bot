#!/bin/bash
# update.sh — обновление арбитражного бота
# Использование: ./update.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

echo "=== Обновление арбитражного бота ==="

# Остановка бота, если запущен
if pgrep -f "arb-bot" > /dev/null; then
    echo "Остановка бота..."
    ./stop.sh
fi

# Обновление кода (если используется git)
if [ -d ".git" ]; then
    echo "Обновление кода из репозитория..."
    git pull
fi

# Обновление зависимостей и пересборка
echo "Обновление зависимостей..."
cargo update

echo "Сборка проекта..."
cargo build --release

echo "=== Обновление завершено ==="
echo "Запустите бота командой: ./start.sh"

