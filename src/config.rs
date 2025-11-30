// config.rs — загрузка и валидация конфигурации
// Назначение: чтение config.toml, .env, проверка корректности параметров

use anyhow::{Result, Context};
use serde::Deserialize;
use std::fs;

#[derive(Debug, Clone, Deserialize)]
pub struct Config {
    pub network: NetworkConfig,
    pub wallet: WalletConfig,
    pub arbitrage: ArbitrageConfig,
    pub dex: DexConfig,
    pub monitoring: MonitoringConfig,
    pub safety: SafetyConfig,
}

#[derive(Debug, Clone, Deserialize)]
pub struct NetworkConfig {
    pub rpc_url: String,
    pub ws_url: String,
    pub commitment: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct WalletConfig {
    pub keypair_path: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ArbitrageConfig {
    pub min_profit_percent: f64,
    pub max_trade_amount_sol: f64,
    pub slippage_tolerance: f64,
    pub transaction_timeout_sec: u64,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DexConfig {
    pub enabled_dexes: Vec<String>,
    pub trading_pairs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct MonitoringConfig {
    pub check_interval_ms: u64,
    pub log_level: String,
    pub log_file: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SafetyConfig {
    pub simulation_mode: bool,
    pub max_consecutive_failures: u32,
    pub min_balance_sol: f64,
}

impl Config {
    /// Загрузка конфигурации из файлов
    pub fn load() -> Result<Self> {
        // Загрузка переменных окружения из .env (если существует)
        dotenv::dotenv().ok();
        
        // Чтение config.toml
        let config_path = "config.toml";
        let config_str = fs::read_to_string(config_path)
            .context(format!("Не удалось прочитать {}", config_path))?;
        
        let config: Config = toml::from_str(&config_str)
            .context("Ошибка парсинга config.toml")?;
        
        // Валидация
        config.validate()?;
        
        Ok(config)
    }
    
    /// Валидация параметров конфигурации
    fn validate(&self) -> Result<()> {
        // Проверка RPC URL
        if !self.network.rpc_url.starts_with("http") {
            return Err(anyhow::anyhow!("Некорректный RPC URL"));
        }
        
        // Проверка минимальной прибыли
        if self.arbitrage.min_profit_percent < 0.0 {
            return Err(anyhow::anyhow!("min_profit_percent должен быть >= 0"));
        }
        
        // Проверка максимальной суммы сделки
        if self.arbitrage.max_trade_amount_sol <= 0.0 {
            return Err(anyhow::anyhow!("max_trade_amount_sol должен быть > 0"));
        }
        
        // Проверка наличия DEX
        if self.dex.enabled_dexes.is_empty() {
            return Err(anyhow::anyhow!("Необходимо указать хотя бы один DEX"));
        }
        
        // Проверка торговых пар
        if self.dex.trading_pairs.is_empty() {
            return Err(anyhow::anyhow!("Необходимо указать хотя бы одну торговую пару"));
        }
        
        Ok(())
    }
}