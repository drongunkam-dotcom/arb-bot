#!/bin/bash
# stop.sh — остановка арбитражного бота
# Использование: ./stop.sh

set -euo pipefail

# Поиск процесса бота
PID=$(pgrep -f "arb-bot" || true)

if [ -z "$PID" ]; then
    echo "Бот не запущен"
    exit 0
fi

echo "Остановка бота (PID: $PID)..."
kill -SIGTERM "$PID"

# Ожидание завершения
sleep 2

# Проверка, завершился ли процесс
if kill -0 "$PID" 2>/dev/null; then
    echo "Принудительное завершение..."
    kill -SIGKILL "$PID"
fi

echo "Бот остановлен"

