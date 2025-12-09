# Решение по стеку веб-интерфейса (Этап 3.1)

## Дата: 2024
## Статус: ✅ Завершено

---

## 1. Выбор фреймворка

### Решение: **axum**

**Обоснование:**
- ✅ Нативная интеграция с tokio (уже используется в проекте)
- ✅ Минималистичный и производительный
- ✅ Встроенная поддержка WebSocket
- ✅ Простая маршрутизация и middleware
- ✅ Активная разработка и поддержка
- ✅ Меньше зависимостей, чем actix-web
- ✅ Лучшая производительность для async/await паттернов

**Альтернативы рассмотрены:**
- ❌ **actix-web**: более тяжёлый, избыточный для наших задач
- ❌ **yew/leptos**: SSR фреймворки, избыточны для простого dashboard
- ❌ **axeler**: нестабильный, недостаточно зрелый

---

## 2. Архитектура

### Решение: **Встроенный сервер в отдельном модуле**

**Структура:**
```
src/
  web/
    mod.rs              # Основной модуль веб-сервера
    server.rs           # HTTP сервер и маршрутизация
    handlers.rs          # Обработчики API endpoints
    websocket.rs         # WebSocket для real-time обновлений
    auth.rs              # Аутентификация и авторизация
    state.rs             # Shared state для доступа к данным бота
    static_files.rs      # Раздача статических файлов
  static/                # Статические файлы (HTML, CSS, JS)
    index.html
    app.js
    styles.css
```

**Обоснование:**
- ✅ Соответствует правилам проекта (отдельный модуль `src/web/`)
- ✅ Проще в развёртывании (один бинарник)
- ✅ Меньше точек отказа
- ✅ Проще управление зависимостями
- ✅ Легче тестирование
- ✅ Меньше overhead на коммуникацию между процессами

**Альтернатива (отдельный сервис):**
- ❌ Сложнее развёртывание
- ❌ Нужна IPC коммуникация
- ❌ Больше точек отказа
- ✅ Плюс: можно перезапускать независимо (но не критично)

---

## 3. Проектирование API endpoints

### 3.1 REST API Endpoints

#### Статус и управление

**GET /api/status**
- Описание: Статус бота (активен/остановлен, режим симуляции)
- Ответ:
```json
{
  "status": "running" | "stopped" | "error",
  "simulation_mode": true,
  "uptime_seconds": 12345,
  "version": "0.1.0"
}
```
- Аутентификация: ✅ Требуется

**GET /api/balance**
- Описание: Баланс кошелька
- Ответ:
```json
{
  "sol_balance": "1.234567",
  "usd_equivalent": "123.45",
  "min_balance_sol": "0.1"
}
```
- Аутентификация: ✅ Требуется

**GET /api/opportunities**
- Описание: Текущие арбитражные возможности
- Параметры запроса:
  - `limit` (опционально, по умолчанию 10): количество возможностей
  - `min_profit` (опционально): минимальная прибыль в процентах
- Ответ:
```json
{
  "opportunities": [
    {
      "from_dex": "raydium",
      "to_dex": "orca",
      "base_token": "SOL",
      "quote_token": "USDC",
      "buy_price": "100.50",
      "sell_price": "101.00",
      "profit_percent": "0.50",
      "profit_percent_after_fees": "0.45",
      "trade_amount": "1.0",
      "estimated_fees": "0.05"
    }
  ],
  "count": 1,
  "timestamp": "2024-01-01T12:00:00Z"
}
```
- Аутентификация: ✅ Требуется

**GET /api/history**
- Описание: История сделок
- Параметры запроса:
  - `limit` (опционально, по умолчанию 50): количество записей
  - `offset` (опционально, по умолчанию 0): смещение для пагинации
  - `from_dex` (опционально): фильтр по DEX источника
  - `status` (опционально): фильтр по статусу (success, failed, simulated)
- Ответ:
```json
{
  "trades": [
    {
      "id": "uuid",
      "timestamp": "2024-01-01T12:00:00Z",
      "from_dex": "raydium",
      "to_dex": "orca",
      "base_token": "SOL",
      "quote_token": "USDC",
      "amount": "1.0",
      "profit_percent": "0.50",
      "profit_sol": "0.005",
      "status": "success" | "failed" | "simulated",
      "tx_signature": "signature..." | null
    }
  ],
  "total": 100,
  "limit": 50,
  "offset": 0
}
```
- Аутентификация: ✅ Требуется

**GET /api/metrics**
- Описание: Метрики производительности
- Ответ:
```json
{
  "total_trades": 100,
  "successful_trades": 95,
  "failed_trades": 5,
  "total_profit_sol": "0.5",
  "total_profit_usd": "50.0",
  "average_profit_percent": "0.45",
  "rpc_calls_count": 1000,
  "rpc_errors_count": 5,
  "average_response_time_ms": 150,
  "last_trade_timestamp": "2024-01-01T12:00:00Z" | null
}
```
- Аутентификация: ✅ Требуется

**GET /api/config**
- Описание: Read-only просмотр конфигурации (без секретов)
- Ответ:
```json
{
  "network": {
    "rpc_url": "https://api.mainnet-beta.solana.com",
    "commitment": "confirmed"
  },
  "arbitrage": {
    "min_profit_percent": "0.5",
    "max_trade_amount_sol": "1.0",
    "slippage_tolerance": "1.0"
  },
  "dex": {
    "enabled_dexes": ["raydium", "orca"],
    "trading_pairs": ["SOL/USDC", "SOL/USDT"]
  },
  "monitoring": {
    "check_interval_ms": 1000,
    "log_level": "info"
  },
  "safety": {
    "simulation_mode": true,
    "max_consecutive_failures": 5,
    "min_balance_sol": "0.1"
  }
}
```
- Аутентификация: ✅ Требуется
- ⚠️ **Важно**: Секреты (ключи, пароли) не возвращаются

**POST /api/control/start**
- Описание: Запуск бота (если остановлен)
- Тело запроса: пустое или `{}`
- Ответ:
```json
{
  "status": "started",
  "message": "Бот запущен"
}
```
- Аутентификация: ✅ Требуется

**POST /api/control/stop**
- Описание: Остановка бота (graceful shutdown)
- Тело запроса: пустое или `{}`
- Ответ:
```json
{
  "status": "stopped",
  "message": "Бот остановлен"
}
```
- Аутентификация: ✅ Требуется

**POST /api/config/reload** (опционально)
- Описание: Перезагрузка конфигурации из файлов
- Тело запроса: пустое или `{}`
- Ответ:
```json
{
  "status": "reloaded",
  "message": "Конфигурация перезагружена"
}
```
- Аутентификация: ✅ Требуется
- ⚠️ **Важно**: Только перезагрузка, не изменение через API

#### Health check

**GET /health**
- Описание: Health check endpoint (для мониторинга)
- Ответ:
```json
{
  "status": "healthy" | "unhealthy",
  "timestamp": "2024-01-01T12:00:00Z"
}
```
- Аутентификация: ❌ Не требуется (публичный endpoint)

### 3.2 WebSocket Endpoints

**WS /ws/updates**
- Описание: Real-time обновления (статус, возможности, сделки, метрики)
- Аутентификация: ✅ Требуется (через query параметр `token` или заголовок)
- Формат сообщений:
```json
{
  "type": "status" | "opportunity" | "trade" | "metrics" | "error",
  "data": { ... },
  "timestamp": "2024-01-01T12:00:00Z"
}
```

**WS /ws/logs**
- Описание: Real-time поток логов
- Аутентификация: ✅ Требуется
- Формат сообщений:
```json
{
  "level": "info" | "warn" | "error" | "debug",
  "message": "Текст лога",
  "timestamp": "2024-01-01T12:00:00Z"
}
```

---

## 4. Аутентификация и безопасность

### Решение: **Basic Authentication** (начальная версия)

**Обоснование:**
- ✅ Простая реализация
- ✅ Достаточно для внутреннего использования
- ✅ Легко заменить на JWT позже

**Реализация:**
- Пароль хранится в `.env` (не в config.toml)
- Используется `Authorization: Basic <base64(username:password)>`
- HTTPS обязателен в продакшене (через reverse proxy или встроенный TLS)

**Будущее улучшение:**
- JWT токены для более безопасной аутентификации
- Session management
- Rate limiting per user

### Дополнительные меры безопасности:

1. **CORS**: Разрешать только с доверенных доменов
2. **Rate Limiting**: Ограничение запросов (например, 100 req/min)
3. **HTTPS**: Обязательно в продакшене (через nginx reverse proxy)
4. **Валидация входных данных**: Все параметры валидируются
5. **Логирование доступа**: Все запросы логируются (без секретов)

---

## 5. Интеграция с существующим кодом

### Shared State

Создаётся структура `WebState` для доступа к данным бота:

```rust
pub struct WebState {
    pub config: Arc<Config>,
    pub monitor: Arc<Monitor>,
    pub arbitrage_engine: Arc<Mutex<Option<ArbitrageEngine>>>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub trade_history: Arc<Mutex<Vec<TradeRecord>>>,
}
```

### Запуск веб-сервера

В `main.rs` веб-сервер запускается в отдельной задаче tokio:

```rust
// Запуск веб-сервера (если включён в конфиге)
if config.web.enabled {
    let web_state = web::create_state(config.clone(), monitor.clone(), arb_engine);
    tokio::spawn(async move {
        if let Err(e) = web::start_server(web_state, config.web.port).await {
            log::error!("Ошибка веб-сервера: {}", e);
        }
    });
}
```

---

## 6. Конфигурация веб-сервера

Добавление в `config.toml`:

```toml
[web]
# Включить веб-интерфейс
enabled = true
# Порт для HTTP сервера
port = 8080
# Адрес для привязки (0.0.0.0 для всех интерфейсов, 127.0.0.1 для локального)
bind_address = "127.0.0.1"
# Путь к статическим файлам
static_dir = "/opt/arb-bot/static"
# Включить HTTPS (требует сертификаты)
https_enabled = false
# Путь к сертификату (если https_enabled = true)
cert_path = "/opt/arb-bot/certs/cert.pem"
# Путь к приватному ключу (если https_enabled = true)
key_path = "/opt/arb-bot/certs/key.pem"
```

Добавление в `.env`:

```env
# Веб-интерфейс: аутентификация
WEB_USERNAME=admin
WEB_PASSWORD=secure_password_here
```

---

## 7. Frontend (начальная версия)

### Решение: **Статические файлы (HTML/CSS/JS)**

**Обоснование:**
- ✅ Простота развёртывания
- ✅ Быстрая разработка MVP
- ✅ Легко заменить на фреймворк позже
- ✅ Минимальные зависимости

**Структура:**
```
static/
  index.html          # Главная страница (dashboard)
  app.js              # Основная логика (API вызовы, WebSocket)
  styles.css          # Стили
  assets/             # Дополнительные ресурсы (иконки, изображения)
```

**Будущее улучшение:**
- React/Vue для более сложного UI
- TypeScript для типобезопасности
- Компонентная архитектура

---

## 8. Зависимости

Добавление в `Cargo.toml`:

```toml
# Web framework
axum = { version = "0.7", features = ["ws", "multipart"] }
tower = "0.4"
tower-http = { version = "0.5", features = ["cors", "compression", "fs"] }

# WebSocket
tokio-tungstenite = "0.21"

# Serialization (уже есть)
serde_json = "1.0"

# Authentication
base64 = "0.21"
bcrypt = "0.15"  # Для хеширования паролей (опционально)

# TLS (если встроенный HTTPS)
rustls = "0.21"
tokio-rustls = "0.24"
```

---

## 9. План реализации

### Этап 3.2: Backend API (Rust)
1. Создать модуль `src/web/`
2. Реализовать базовую структуру сервера
3. Реализовать все REST API endpoints
4. Реализовать WebSocket для real-time обновлений
5. Реализовать аутентификацию
6. Добавить конфигурацию в config.toml

### Этап 3.3: Frontend
1. Создать базовый HTML/CSS/JS
2. Реализовать dashboard с метриками
3. Реализовать таблицу возможностей
4. Реализовать историю сделок
5. Интегрировать WebSocket для real-time обновлений

### Этап 3.4: Безопасность
1. Настроить HTTPS (через nginx или встроенный)
2. Реализовать rate limiting
3. Добавить CORS настройки
4. Реализовать логирование доступа
5. Security audit

---

## 10. Итоговое решение

✅ **Backend**: axum (встроенный сервер в модуле `src/web/`)
✅ **Frontend**: Статические файлы (HTML/CSS/JS)
✅ **WebSocket**: tokio-tungstenite через axum
✅ **Аутентификация**: Basic Auth (начальная версия)
✅ **Архитектура**: Встроенный сервер, отдельный модуль

**Преимущества:**
- Простота развёртывания
- Минимальные зависимости
- Хорошая производительность
- Легко расширять
- Соответствует правилам проекта

**Следующие шаги:**
- Реализация backend API (Этап 3.2)
- Реализация frontend (Этап 3.3)
- Настройка безопасности (Этап 3.4)

