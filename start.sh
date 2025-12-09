#!/bin/bash
# start.sh — запуск арбитражного бота
# Использование: ./start.sh

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
cd "$SCRIPT_DIR"

# Проверка наличия конфигурации
if [ ! -f "config.toml" ]; then
    echo "Ошибка: config.toml не найден"
    echo "Скопируйте config.example.toml в config.toml и настройте его"
    exit 1
fi

# Проверка режима симуляции
SIMULATION_MODE=$(grep -E "^simulation_mode\s*=" config.toml | cut -d'=' -f2 | tr -d ' "')
if [ "$SIMULATION_MODE" != "true" ]; then
    echo "⚠️  ВНИМАНИЕ: Режим симуляции отключен! Реальные транзакции будут выполняться!"
    read -p "Продолжить? (yes/no): " confirm
    if [ "$confirm" != "yes" ]; then
        exit 1
    fi
fi

# Запуск бота
echo "Запуск арбитражного бота..."
RUST_LOG=info cargo run --release

