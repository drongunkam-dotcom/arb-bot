#!/bin/bash
# install.sh — автоматическая установка окружения для арбитражного бота Solana
# Требования: Ubuntu 22.04, права sudo
# Назначение: установка Rust, зависимостей, создание структуры каталогов

set -euo pipefail

echo "=== Установка системных зависимостей ==="
sudo apt-get update
sudo apt-get install -y \
    build-essential \
    pkg-config \
    libssl-dev \
    libudev-dev \
    curl \
    git \
    jq

echo "=== Установка Rust (версия 1.75.0) ==="
if ! command -v rustc &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain 1.75.0
    source "$HOME/.cargo/env"
else
    echo "Rust уже установлен: $(rustc --version)"
fi

echo "=== Создание структуры каталогов ==="
sudo mkdir -p /opt/arb-bot/{src,tests,keys,logs}
sudo mkdir -p /var/log/arb-bot
sudo chown -R $USER:$USER /opt/arb-bot
sudo chown -R $USER:$USER /var/log/arb-bot

echo "=== Установка завершена ==="
echo "Следующие шаги:"
echo "1. Скопируйте все файлы проекта в /opt/arb-bot/"
echo "2. Создайте config.toml из config.example.toml"
echo "3. Создайте .env из .env.example и заполните секреты"
echo "4. Запустите: cd /opt/arb-bot && cargo build --release"

