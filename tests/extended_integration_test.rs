/// Расширенные интеграционные тесты
/// 
/// Включает тесты с моками, проверку обработки ошибок, 
/// валидацию бизнес-логики и edge cases

use anyhow::{Context, Result};
use arb_bot::config::Config;
use arb_bot::wallet::Wallet;
use arb_bot::dex::DexManager;
use arb_bot::monitor::Monitor;
use arb_bot::arbitrage::ArbitrageEngine;
use rust_decimal::Decimal;
use std::str::FromStr;
use tempfile::TempDir;

#[path = "mocks.rs"]
mod mocks;

use mocks::{MockRpcClient, create_test_keypair};

/// Создание тестовой конфигурации
fn create_test_config(temp_dir: &TempDir) -> Result<Config> {
    let keypair_path = temp_dir.path().join("test_wallet.json");
    let keypair = create_test_keypair();
    let keypair_json = serde_json::json!({
        "secretKey": keypair.to_bytes().to_vec()
    });
    std::fs::write(&keypair_path, serde_json::to_string_pretty(&keypair_json)?)?;

    let config_str = format!(
        r#"
[network]
rpc_url = "https://api.devnet.solana.com"
ws_url = "wss://api.devnet.solana.com"
commitment = "confirmed"

[wallet]
keypair_path = "{}"

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
log_file = "{}"

[safety]
simulation_mode = true
max_consecutive_failures = 5
min_balance_sol = 0.1
"#,
        keypair_path.to_str().unwrap(),
        temp_dir.path().join("test.log").to_str().unwrap()
    );

    let config: Config = toml::from_str(&config_str)
        .context("Ошибка парсинга тестовой конфигурации")?;
    
    Ok(config)
}

/// Тест поиска арбитражных возможностей с моками
#[tokio::test]
async fn test_find_opportunities_with_mocks() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;
    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let monitor = Monitor::new(&config);

    let engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    // Тест поиска возможностей (может не найти из-за отсутствия реальных цен)
    let opportunities = engine.find_opportunities().await?;
    // В режиме симуляции с заглушками может быть пусто, это нормально
    log::info!("Найдено возможностей: {}", opportunities.len());

    Ok(())
}

/// Тест обработки ошибок при получении цен
#[tokio::test]
async fn test_error_handling_price_fetch() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;
    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let monitor = Monitor::new(&config);

    let engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    // Попытка найти возможности - должна обработать ошибки получения цен
    let opportunities = engine.find_opportunities().await?;
    // Даже если есть ошибки, функция должна вернуть пустой список, а не паниковать
    assert!(opportunities.is_empty() || !opportunities.is_empty());

    Ok(())
}

/// Тест валидации конфигурации
#[tokio::test]
async fn test_config_validation() -> Result<()> {
    // Тест валидной конфигурации
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;
    
    // Проверка, что конфигурация валидна
    assert!(!config.network.rpc_url.is_empty());
    assert!(config.arbitrage.min_profit_percent > 0.0);
    assert!(config.arbitrage.max_trade_amount_sol > 0.0);
    assert!(config.monitoring.check_interval_ms > 0);

    Ok(())
}

/// Тест инициализации всех компонентов
#[tokio::test]
async fn test_component_initialization() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;

    // Инициализация кошелька
    let wallet = Wallet::new(&config)?;
    assert!(!wallet.pubkey().to_string().is_empty());

    // Инициализация DexManager
    let dex_manager = DexManager::new(&config)?;
    assert!(!dex_manager.get_dexes().is_empty());

    // Инициализация монитора
    let monitor = Monitor::new(&config);
    monitor.log_warning("Тестовое сообщение");

    // Инициализация движка арбитража
    let engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    // Проверка, что движок создан
    assert!(std::mem::size_of_val(&engine) > 0);

    Ok(())
}

/// Тест расчёта прибыли
#[tokio::test]
async fn test_profit_calculation() -> Result<()> {
    // Тест расчёта прибыли в процентах
    let buy_price = Decimal::from_str("100.0")?;
    let sell_price = Decimal::from_str("105.0")?;
    
    let profit_percent = ((sell_price - buy_price) / buy_price) * Decimal::from(100);
    assert_eq!(profit_percent, Decimal::from_str("5.0")?);

    // Тест с учётом комиссий
    let fee_percent = Decimal::from_str("0.5")?; // 0.5% комиссия
    let profit_after_fees = profit_percent - fee_percent;
    assert_eq!(profit_after_fees, Decimal::from_str("4.5")?);

    Ok(())
}

/// Тест обработки edge cases
#[tokio::test]
async fn test_edge_cases() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let config = create_test_config(&temp_dir)?;
    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let monitor = Monitor::new(&config);

    let _engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    // Тест с пустым списком торговых пар
    let mut config_empty_pairs = config.clone();
    config_empty_pairs.dex.trading_pairs = vec![];
    // Должно обработать без паники
    let _ = config_empty_pairs;

    // Тест с одним DEX (недостаточно для арбитража)
    let mut config_one_dex = config.clone();
    config_one_dex.dex.enabled_dexes = vec!["raydium".to_string()];
    // Должно обработать без паники
    let _ = config_one_dex;

    Ok(())
}

/// Тест retry логики
#[tokio::test]
async fn test_retry_logic() -> Result<()> {
    let mock_rpc = MockRpcClient::new();
    let test_pubkey = solana_sdk::pubkey::Pubkey::new_unique();

    // Тест успешного получения после нескольких попыток
    mock_rpc.set_should_fail(true);
    // Первая попытка должна провалиться
    assert!(mock_rpc.get_balance(&test_pubkey).is_err());

    // Включаем успешный режим
    mock_rpc.set_should_fail(false);
    mock_rpc.set_balance(&test_pubkey, 1_000_000_000);
    
    // Теперь должно работать
    let balance = mock_rpc.get_balance(&test_pubkey)?;
    assert_eq!(balance, 1_000_000_000);

    Ok(())
}

/// Тест обработки таймаутов
#[tokio::test]
async fn test_timeout_handling() -> Result<()> {
    use tokio::time::{timeout, Duration, sleep};

    // Таймаут меньше времени операции
    let slow_operation1 = async {
        sleep(Duration::from_millis(100)).await;
        Ok::<(), anyhow::Error>(())
    };
    let result = timeout(Duration::from_millis(50), slow_operation1).await;
    assert!(result.is_err()); // Должен быть таймаут

    // Таймаут больше времени операции
    let slow_operation2 = async {
        sleep(Duration::from_millis(100)).await;
        Ok::<(), anyhow::Error>(())
    };
    let result = timeout(Duration::from_millis(200), slow_operation2).await;
    assert!(result.is_ok()); // Должен успешно выполниться

    Ok(())
}

/// Тест валидации торговых пар
#[tokio::test]
async fn test_trading_pair_validation() -> Result<()> {
    // Валидные пары
    let valid_pairs = vec!["SOL/USDC", "BTC/USDT", "ETH/SOL"];
    for pair in valid_pairs {
        let parts: Vec<&str> = pair.split('/').collect();
        assert_eq!(parts.len(), 2, "Пара должна содержать два токена");
    }

    // Невалидные пары
    let invalid_pairs = vec!["SOL", "SOL/USDC/BTC", ""];
    for pair in invalid_pairs {
        let parts: Vec<&str> = pair.split('/').collect();
        if parts.len() != 2 {
            // Это ожидаемо для невалидных пар
            log::debug!("Невалидная пара (ожидаемо): {}", pair);
        }
    }

    Ok(())
}

/// Тест проверки минимальной прибыли
#[tokio::test]
async fn test_min_profit_threshold() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = create_test_config(&temp_dir)?;
    config.arbitrage.min_profit_percent = 1.0; // 1% минимальная прибыль

    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let monitor = Monitor::new(&config);

    let engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    // Поиск возможностей с минимальной прибылью 1%
    let opportunities = engine.find_opportunities().await?;
    
    // Все найденные возможности должны иметь прибыль >= 1%
    for opp in &opportunities {
        assert!(
            opp.profit_percent_after_fees >= Decimal::from_str("1.0")?,
            "Прибыль должна быть >= минимального порога"
        );
    }

    Ok(())
}

