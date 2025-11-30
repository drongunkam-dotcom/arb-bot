# Solana Arbitrage Bot

Арбитражный бот для поиска и выполнения арбитражных сделок на DEX Solana (Raydium, Orca).

## Структура проекта

- `src/` — исходный код бота на Rust
- `tests/` — интеграционные тесты
- `scripts/` — скрипты для деплоя и управления на VPS
- `web/` — веб-интерфейс для управления ботом
- `keys/` — приватные ключи (не коммитятся в git)
- `logs/` — логи работы бота
- `backups/` — резервные копии ключей

## Быстрый старт (локально)

1. Установи Rust: https://rustup.rs
2. Скопируй конфиги:
   ```bash
   cp config.example.toml config.toml
   cp .env.example .env