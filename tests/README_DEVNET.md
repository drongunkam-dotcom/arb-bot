# Тестирование на Devnet

## Описание

Набор тестов для проверки интеграции с Raydium на devnet окружении Solana.

## Требования

1. **Доступ к devnet RPC**: `https://api.devnet.solana.com`
2. **Тестовый кошелёк** (создаётся автоматически в тестах)
3. **Баланс на devnet** (можно получить через airdrop)

## Запуск тестов

### Все тесты (игнорируемые)

```bash
cargo test --test devnet_test -- --ignored
```

### Отдельные тесты

```bash
# Тест подключения к RPC
cargo test --test devnet_test test_devnet_rpc_connection -- --ignored

# Тест инициализации кошелька
cargo test --test devnet_test test_devnet_wallet_initialization -- --ignored

# Тест инициализации DexManager
cargo test --test devnet_test test_devnet_dex_manager_initialization -- --ignored

# Тест получения цены
cargo test --test devnet_test test_devnet_raydium_get_price -- --ignored

# Тест симуляции свопа
cargo test --test devnet_test test_devnet_raydium_swap_simulation -- --ignored

# Комплексный тест
cargo test --test devnet_test test_devnet_full_integration -- --ignored
```

## Получение тестовых SOL на devnet

Если баланс кошелька равен нулю, можно запросить airdrop:

```bash
# Установите Solana CLI, если ещё не установлен
# https://docs.solana.com/cli/install-solana-cli-tools

# Переключитесь на devnet
solana config set --url devnet

# Запросите airdrop (максимум 2 SOL за раз)
solana airdrop 2 <PUBKEY> --url devnet
```

## Ожидаемые результаты

### ✅ Должны пройти:
- `test_devnet_rpc_connection` - подключение к devnet RPC
- `test_devnet_wallet_initialization` - инициализация кошелька
- `test_devnet_dex_manager_initialization` - инициализация DexManager
- `test_devnet_raydium_swap_simulation` - симуляция свопа (всегда в режиме симуляции)

### ⚠️ Могут не пройти (из-за заглушек):
- `test_devnet_raydium_get_price` - получение цены (требует реальные адреса пулов)
- Реальные свопы (не реализованы полностью)

## Известные ограничения

1. **Заглушки в get_pool_address**: Функция `get_pool_address` в `src/dex.rs` возвращает заглушку вместо реальных адресов пулов. Для полноценного тестирования нужно:
   - Реализовать получение адресов пулов через Raydium API
   - Или использовать известные адреса пулов на devnet

2. **Парсинг данных пула**: Функция `get_pool_data` использует упрощённый парсинг. Для реального тестирования нужно:
   - Реализовать полный парсинг структуры пула Raydium
   - Правильно читать резервы из vault аккаунтов

3. **Построение инструкций swap**: Инструкции swap построены упрощённо и могут не работать с реальными пулами.

## Следующие шаги

Для завершения тестирования на devnet (шаг 2.1 roadmap):

1. ✅ Созданы тесты для devnet
2. ⏳ Реализовать получение реальных адресов пулов Raydium на devnet
3. ⏳ Реализовать правильный парсинг данных пула
4. ⏳ Протестировать реальные свопы на devnet (с тестовыми токенами)

## Логирование

Тесты используют `env_logger` с уровнем `debug`. Для более детального логирования:

```bash
RUST_LOG=debug cargo test --test devnet_test -- --ignored
```

