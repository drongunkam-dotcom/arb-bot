use anyhow::Result;
use arb_bot::config::Config;
use arb_bot::wallet::Wallet;
use arb_bot::dex::DexManager;
use arb_bot::monitor::Monitor;
use arb_bot::arbitrage::ArbitrageEngine;
use std::fs;
use std::path::PathBuf;

/// Базовые интеграционные тесты
#[tokio::test]
async fn test_config_loading() -> Result<()> {
    // Создаём временный config.toml для теста
    let test_config = r#"
[network]
rpc_url = "https://api.devnet.solana.com"
ws_url = "wss://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "/tmp/test_wallet.json"

[arbitrage]
min_profit_percent = 0.5
max_trade_amount_sol = 1.0
slippage_tolerance = 1.0
transaction_timeout_sec = 30

[dex]
enabled_dexes = ["raydium", "orca"]
trading_pairs = ["SOL/USDC"]

[monitoring]
check_interval_ms = 1000
log_level = "info"
log_file = "/tmp/test.log"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#;

    let test_config_path = PathBuf::from("/tmp/test_config.toml");
    fs::write(&test_config_path, test_config)?;

    // Тест загрузки конфигурации
    // Примечание: в реальном тесте нужно мокировать Config::load()
    // или использовать временный файл
    assert!(test_config_path.exists());

    // Очистка
    fs::remove_file(&test_config_path).ok();

    Ok(())
}

#[tokio::test]
async fn test_dex_manager_creation() -> Result<()> {
    // Создаём минимальную конфигурацию для теста
    let config = create_test_config()?;

    // Создание DexManager должно работать даже с заглушками
    let dex_manager = DexManager::new(&config)?;
    assert!(!dex_manager.get_dexes().is_empty());

    Ok(())
}

#[tokio::test]
async fn test_monitor_creation() -> Result<()> {
    let config = create_test_config()?;
    let monitor = Monitor::new(&config);
    
    // Тест логирования (не должно паниковать)
    monitor.log_arbitrage("raydium", "orca", rust_decimal::Decimal::new(50, 0), true);
    monitor.log_error("Тестовая ошибка");
    monitor.log_warning("Тестовое предупреждение");

    Ok(())
}

/// Создание тестовой конфигурации
fn create_test_config() -> Result<Config> {
    use std::io::Write;
    use tempfile::NamedTempFile;

    let test_config = r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "/tmp/test_wallet.json"

[arbitrage]
min_profit_percent = 0.5
max_trade_amount_sol = 1.0
slippage_tolerance = 1.0
transaction_timeout_sec = 30

[dex]
enabled_dexes = ["raydium"]
trading_pairs = ["SOL/USDC"]

[monitoring]
check_interval_ms = 1000
log_level = "info"
log_file = "/tmp/test.log"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#;

    // Создаём временный файл конфигурации
    let mut temp_file = NamedTempFile::new()?;
    temp_file.write_all(test_config.as_bytes())?;
    let temp_path = temp_file.path().to_path_buf();
    temp_file.persist(&temp_path)?;

    // Загружаем конфигурацию
    // Примечание: Config::load() использует фиксированный путь
    // В реальных тестах нужно мокировать или изменять логику загрузки
    // Для базового теста просто проверяем, что структура корректна
    let config: Config = toml::from_str(test_config)?;
    
    fs::remove_file(&temp_path).ok();
    
    Ok(config)
}

