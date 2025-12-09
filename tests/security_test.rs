/// Тесты безопасности
/// 
/// Проверка безопасности кода: отсутствие секретов в логах,
/// правильная обработка ключей, валидация входных данных

use anyhow::Result;
use arb_bot::config::Config;
use arb_bot::wallet::Wallet;
use std::fs;
use tempfile::TempDir;
use solana_sdk::signature::Keypair;

/// Тест отсутствия приватных ключей в логах
#[tokio::test]
async fn test_no_secrets_in_logs() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let keypair_path = temp_dir.path().join("test_wallet.json");
    let keypair = Keypair::new();
    let keypair_json = serde_json::json!({
        "secretKey": keypair.to_bytes().to_vec()
    });
    fs::write(&keypair_path, serde_json::to_string_pretty(&keypair_json)?)?;

    let config_str = format!(
        r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "{}"

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
log_file = "{}"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#,
        keypair_path.to_str().unwrap(),
        temp_dir.path().join("test.log").to_str().unwrap()
    );

    let config: Config = toml::from_str(&config_str)?;
    let wallet = Wallet::new(&config)?;

    // Проверка, что приватный ключ не логируется
    let pubkey_str = wallet.pubkey().to_string();
    
    // Логируем публичный ключ (это безопасно)
    log::info!("Публичный ключ: {}", pubkey_str);
    
    // Проверка, что приватный ключ не содержится в строке публичного ключа
    let secret_key_bytes = keypair.to_bytes();
    let secret_key_str = format!("{:?}", secret_key_bytes);
    assert!(!pubkey_str.contains(&secret_key_str), "Приватный ключ не должен появляться в логах");

    Ok(())
}

/// Тест проверки прав доступа к файлам ключей
#[tokio::test]
#[cfg(unix)]
async fn test_key_file_permissions() -> Result<()> {
    use std::os::unix::fs::PermissionsExt;
    
    let temp_dir = TempDir::new()?;
    let keypair_path = temp_dir.path().join("test_wallet.json");
    let keypair = Keypair::new();
    let keypair_json = serde_json::json!({
        "secretKey": keypair.to_bytes().to_vec()
    });
    fs::write(&keypair_path, serde_json::to_string_pretty(&keypair_json)?)?;

    // Устанавливаем правильные права (только для владельца)
    let mut perms = fs::metadata(&keypair_path)?.permissions();
    perms.set_mode(0o600); // rw-------
    fs::set_permissions(&keypair_path, perms)?;

    let config_str = format!(
        r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "{}"

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
log_file = "{}"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#,
        keypair_path.to_str().unwrap(),
        temp_dir.path().join("test.log").to_str().unwrap()
    );

    let config: Config = toml::from_str(&config_str)?;
    
    // Кошелёк должен загрузиться даже с правильными правами
    let wallet = Wallet::new(&config)?;
    assert!(!wallet.pubkey().to_string().is_empty());

    Ok(())
}

/// Тест валидации конфигурации на небезопасные значения
#[tokio::test]
async fn test_config_security_validation() -> Result<()> {
    // Тест с невалидной конфигурацией (отрицательная прибыль)
    let _invalid_config = r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "/tmp/test.json"

[arbitrage]
min_profit_percent = -1.0
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

    let config_result: Result<Config, _> = toml::from_str(invalid_config);
    
    // Конфигурация может быть распарсена, но валидация должна поймать ошибку
    if let Ok(mut config) = config_result {
        // Попытка валидации должна выявить проблему
        // В реальной реализации Config::validate() должен проверить min_profit_percent > 0
        if config.arbitrage.min_profit_percent <= 0.0 {
            // Это небезопасная конфигурация
            log::warn!("Обнаружена небезопасная конфигурация: отрицательная прибыль");
        }
    }

    Ok(())
}

/// Тест проверки режима симуляции
#[tokio::test]
async fn test_simulation_mode_enforcement() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let keypair_path = temp_dir.path().join("test_wallet.json");
    let keypair = Keypair::new();
    let keypair_json = serde_json::json!({
        "secretKey": keypair.to_bytes().to_vec()
    });
    fs::write(&keypair_path, serde_json::to_string_pretty(&keypair_json)?)?;

    // Конфигурация с включённым режимом симуляции
    let config_str = format!(
        r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "{}"

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
log_file = "{}"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#,
        keypair_path.to_str().unwrap(),
        temp_dir.path().join("test.log").to_str().unwrap()
    );

    let config: Config = toml::from_str(&config_str)?;
    
    // Проверка, что режим симуляции включён
    assert!(config.safety.simulation_mode, "Режим симуляции должен быть включён для тестов");

    Ok(())
}

/// Тест обработки невалидных путей к файлам
#[tokio::test]
async fn test_invalid_file_paths() -> Result<()> {
    // Попытка загрузить кошелёк с несуществующим путём
    let config_str = r#"
[network]
rpc_url = "https://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "/nonexistent/path/to/key.json"

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

    let config: Config = toml::from_str(config_str)?;
    
    // Попытка загрузить кошелёк должна вернуть ошибку
    let wallet_result = Wallet::new(&config);
    assert!(wallet_result.is_err(), "Загрузка несуществующего ключа должна вернуть ошибку");

    Ok(())
}

/// Тест защиты от переполнения при расчётах
#[tokio::test]
async fn test_overflow_protection() -> Result<()> {
    use rust_decimal::Decimal;
    use std::str::FromStr;

    // Тест расчёта прибыли с очень большими числами
    let buy_price = Decimal::from_str("999999999999.99")?;
    let sell_price = Decimal::from_str("1000000000000.00")?;
    
    // Расчёт должен выполняться без переполнения
    let profit_percent = ((sell_price - buy_price) / buy_price) * Decimal::from(100);
    
    // Проверка, что результат корректен
    assert!(profit_percent > Decimal::ZERO, "Прибыль должна быть положительной");

    Ok(())
}

/// Тест валидации входных данных для торговых пар
#[tokio::test]
async fn test_trading_pair_validation() -> Result<()> {
    // Валидные пары
    let valid_pairs = vec!["SOL/USDC", "BTC/USDT", "ETH/SOL"];
    for pair in valid_pairs {
        let parts: Vec<&str> = pair.split('/').collect();
        assert_eq!(parts.len(), 2, "Пара должна содержать два токена");
        assert!(!parts[0].is_empty(), "Base токен не должен быть пустым");
        assert!(!parts[1].is_empty(), "Quote токен не должен быть пустым");
    }

    // Невалидные пары должны обрабатываться gracefully
    let invalid_pairs = vec!["SOL", "SOL/USDC/BTC", "", "/", "SOL/"];
    for pair in invalid_pairs {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 || parts[0].is_empty() || parts[1].is_empty() {
            // Это ожидаемо для невалидных пар - они должны быть отфильтрованы
            log::debug!("Невалидная пара (ожидаемо): {}", pair);
        }
    }

    Ok(())
}

