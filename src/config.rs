use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Конфигурация приложения
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
    pub arbitrage: ArbitrageConfig,
    pub dex: DexConfig,
    pub monitoring: MonitoringConfig,
    pub safety: SafetyConfig,
    #[serde(default)]
    pub web: WebConfig,
}

/// Настройки сети
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    /// RPC endpoint Solana
    pub rpc_url: String,
    /// WebSocket URL для подписок
    pub ws_url: Option<String>,
    /// Уровень подтверждения транзакций
    pub commitment: String,
}

/// Настройки кошелька
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WalletConfig {
    /// Путь к файлу ключа
    pub keypair_path: PathBuf,
}

/// Настройки арбитража
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ArbitrageConfig {
    /// Минимальная прибыль в процентах
    pub min_profit_percent: f64,
    /// Максимальный объём сделки в SOL
    pub max_trade_amount_sol: f64,
    /// Допустимое проскальзывание в процентах
    pub slippage_tolerance: f64,
    /// Таймаут транзакции в секундах
    pub transaction_timeout_sec: u64,
}

/// Настройки DEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DexConfig {
    /// Список активированных DEX
    pub enabled_dexes: Vec<String>,
    /// Торговые пары для мониторинга
    pub trading_pairs: Vec<String>,
}

/// Настройки мониторинга
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MonitoringConfig {
    /// Интервал проверки в миллисекундах
    pub check_interval_ms: u64,
    /// Уровень логирования
    pub log_level: String,
    /// Путь к файлу логов
    pub log_file: PathBuf,
}

/// Настройки безопасности
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SafetyConfig {
    /// Режим симуляции (не выполняет реальные транзакции)
    pub simulation_mode: bool,
    /// Максимальное количество последовательных ошибок
    pub max_consecutive_failures: u32,
    /// Минимальный баланс SOL для продолжения работы
    pub min_balance_sol: f64,
}

/// Настройки веб-сервера
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WebConfig {
    /// Включить веб-интерфейс
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Порт для HTTP сервера
    #[serde(default = "default_web_port")]
    pub port: u16,
    /// Адрес для привязки
    #[serde(default = "default_bind_address")]
    pub bind_address: String,
    /// Путь к статическим файлам
    #[serde(default = "default_static_dir")]
    pub static_dir: PathBuf,
}

fn default_true() -> bool {
    true
}

fn default_web_port() -> u16 {
    8080
}

fn default_bind_address() -> String {
    "127.0.0.1".to_string()
}

fn default_static_dir() -> PathBuf {
    if cfg!(windows) {
        PathBuf::from("static")
    } else {
        PathBuf::from("/opt/arb-bot/static")
    }
}

impl Config {
    /// Загрузка конфигурации из файла
    pub fn load() -> Result<Self> {
        // Путь к config.toml согласно правилам проекта
        let config_path = if cfg!(windows) {
            // На Windows используем текущую директорию для разработки
            PathBuf::from("config.toml")
        } else {
            PathBuf::from("/opt/arb-bot/config.toml")
        };

        // Загрузка переменных окружения из .env
        dotenv::dotenv().ok();

        // Чтение и парсинг config.toml
        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Не удалось прочитать конфигурацию: {:?}", config_path))?;

        let mut config: Config = toml::from_str(&content)
            .context("Ошибка парсинга config.toml")?;

        // Валидация конфигурации
        config.validate()?;

        Ok(config)
    }

    /// Валидация конфигурации
    fn validate(&self) -> Result<()> {
        // Проверка обязательных полей
        if self.network.rpc_url.is_empty() {
            anyhow::bail!("rpc_url не может быть пустым");
        }

        if self.arbitrage.min_profit_percent <= 0.0 {
            anyhow::bail!("min_profit_percent должен быть больше 0");
        }

        if self.arbitrage.max_trade_amount_sol <= 0.0 {
            anyhow::bail!("max_trade_amount_sol должен быть больше 0");
        }

        if self.monitoring.check_interval_ms == 0 {
            anyhow::bail!("check_interval_ms должен быть больше 0");
        }

        if self.safety.simulation_mode {
            log::warn!("⚠️  Режим симуляции активен - реальные транзакции не выполняются");
        }

        Ok(())
    }
}

