#!/bin/bash
# backup_keys.sh — резервное копирование ключей
# Использование: ./backup_keys.sh [destination]

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
KEYS_DIR="${SCRIPT_DIR}/keys"
BACKUP_DIR="${1:-${SCRIPT_DIR}/backups}"

# Создание директории для бэкапов
mkdir -p "$BACKUP_DIR"

# Генерация имени файла бэкапа с датой
BACKUP_FILE="${BACKUP_DIR}/keys_backup_$(date +%Y%m%d_%H%M%S).tar.gz"

# Проверка наличия ключей
if [ ! -d "$KEYS_DIR" ] || [ -z "$(ls -A "$KEYS_DIR" 2>/dev/null)" ]; then
    echo "Ошибка: директория keys пуста или не существует"
    exit 1
fi

echo "Создание резервной копии ключей..."
tar -czf "$BACKUP_FILE" -C "$SCRIPT_DIR" keys/

# Установка безопасных прав доступа
chmod 600 "$BACKUP_FILE"

echo "✅ Резервная копия создана: $BACKUP_FILE"
echo "⚠️  ВАЖНО: Храните бэкап в безопасном месте!"

