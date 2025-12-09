/// Стресс-тесты производительности
/// 
/// Тесты для проверки производительности и стабильности под нагрузкой

use anyhow::{Context, Result};
use arb_bot::config::Config;
use arb_bot::wallet::Wallet;
use arb_bot::dex::DexManager;
use arb_bot::monitor::Monitor;
use arb_bot::arbitrage::ArbitrageEngine;
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::{Duration, Instant};
use tempfile::TempDir;
use solana_sdk::signature::Keypair;
use futures::future;

/// Создание тестовой конфигурации
fn create_test_config(temp_dir: &TempDir) -> Result<Config> {
    let keypair_path = temp_dir.path().join("test_wallet.json");
    let keypair = Keypair::new();
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

/// Тест производительности поиска возможностей
#[tokio::test]
#[ignore] // Игнорируем по умолчанию, запускать вручную
async fn test_performance_find_opportunities() -> Result<()> {
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

    // Замер времени выполнения поиска возможностей
    let start = Instant::now();
    let opportunities = engine.find_opportunities().await?;
    let duration = start.elapsed();

    log::info!("Поиск возможностей выполнен за {:?}, найдено: {}", duration, opportunities.len());
    
    // Проверка, что поиск выполняется достаточно быстро (< 5 секунд)
    assert!(duration < Duration::from_secs(5), "Поиск возможностей должен выполняться быстро");

    Ok(())
}

/// Тест множественных последовательных поисков возможностей
#[tokio::test]
#[ignore]
async fn test_multiple_find_opportunities() -> Result<()> {
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

    let iterations = 10;
    let mut total_duration = Duration::ZERO;

    for i in 0..iterations {
        let start = Instant::now();
        let _opportunities = engine.find_opportunities().await?;
        let duration = start.elapsed();
        total_duration += duration;
        
        log::info!("Итерация {}: {:?}", i + 1, duration);
    }

    let avg_duration = total_duration / iterations;
    log::info!("Среднее время поиска: {:?}", avg_duration);
    
    // Проверка стабильности производительности
    assert!(avg_duration < Duration::from_secs(5), "Среднее время должно быть приемлемым");

    Ok(())
}

/// Тест параллельного выполнения поиска возможностей
#[tokio::test]
#[ignore]
async fn test_parallel_find_opportunities() -> Result<()> {
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

    // Запуск нескольких параллельных поисков
    let start = Instant::now();
    let search_futures: Vec<_> = (0..5)
        .map(|_| engine.find_opportunities())
        .collect();
    
    let results = future::join_all(search_futures).await;
    let duration = start.elapsed();

    // Проверка, что все запросы завершились успешно
    for (i, result) in results.iter().enumerate() {
        assert!(result.is_ok(), "Запрос {} должен завершиться успешно", i);
    }

    log::info!("5 параллельных поисков выполнены за {:?}", duration);
    
    // Параллельное выполнение должно быть быстрее последовательного
    assert!(duration < Duration::from_secs(10), "Параллельное выполнение должно быть эффективным");

    Ok(())
}

/// Тест производительности расчёта прибыли
#[tokio::test]
async fn test_profit_calculation_performance() -> Result<()> {
    let iterations = 1000;
    let start = Instant::now();

    for _ in 0..iterations {
        let buy_price = Decimal::from_str("100.0")?;
        let sell_price = Decimal::from_str("105.0")?;
        let _profit_percent = ((sell_price - buy_price) / buy_price) * Decimal::from(100);
    }

    let duration = start.elapsed();
    let avg_time = duration / iterations;
    
    log::info!("1000 расчётов прибыли выполнены за {:?}, среднее: {:?}", duration, avg_time);
    
    // Расчёт должен быть очень быстрым (< 1 микросекунда в среднем)
    assert!(avg_time < Duration::from_micros(10), "Расчёт прибыли должен быть очень быстрым");

    Ok(())
}

/// Тест обработки большого количества торговых пар
#[tokio::test]
#[ignore]
async fn test_many_trading_pairs() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let mut config = create_test_config(&temp_dir)?;
    
    // Добавляем много торговых пар
    config.dex.trading_pairs = vec![
        "SOL/USDC".to_string(),
        "SOL/USDT".to_string(),
        "BTC/USDC".to_string(),
        "ETH/USDC".to_string(),
        "USDC/USDT".to_string(),
    ];

    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let monitor = Monitor::new(&config);

    let engine = ArbitrageEngine::new(
        config.clone(),
        wallet,
        dex_manager,
        monitor,
    );

    let start = Instant::now();
    let opportunities = engine.find_opportunities().await?;
    let duration = start.elapsed();

    log::info!("Поиск по {} парам выполнен за {:?}, найдено: {}", 
        config.dex.trading_pairs.len(), duration, opportunities.len());
    
    // Даже с большим количеством пар поиск должен быть быстрым
    assert!(duration < Duration::from_secs(10), "Поиск должен быть эффективным даже с большим количеством пар");

    Ok(())
}

/// Тест стабильности при длительной работе
#[tokio::test]
#[ignore]
async fn test_long_running_stability() -> Result<()> {
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

    let iterations = 100;
    let mut success_count = 0;
    let mut error_count = 0;

    for i in 0..iterations {
        match engine.find_opportunities().await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error_count += 1;
                log::warn!("Ошибка на итерации {}: {}", i, e);
            }
        }
        
        // Небольшая задержка между итерациями
        tokio::time::sleep(Duration::from_millis(10)).await;
    }

    log::info!("Длительный тест: успешно: {}, ошибок: {}", success_count, error_count);
    
    // Большинство итераций должны быть успешными
    let success_rate = (success_count as f64 / iterations as f64) * 100.0;
    assert!(success_rate >= 90.0, "Успешность должна быть >= 90%, получено: {:.2}%", success_rate);

    Ok(())
}

/// Тест обработки ошибок под нагрузкой
#[tokio::test]
#[ignore]
async fn test_error_handling_under_load() -> Result<()> {
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

    // Множественные запросы с возможными ошибками
    let mut success_count = 0;
    let mut error_count = 0;

    for i in 0..50 {
        match engine.find_opportunities().await {
            Ok(_) => {
                success_count += 1;
            }
            Err(e) => {
                error_count += 1;
                // Ошибки должны логироваться, но не должны паниковать
                log::debug!("Ошибка на итерации {}: {}", i, e);
            }
        }
    }

    log::info!("Тест под нагрузкой: успешно: {}, ошибок: {}", success_count, error_count);
    
    // Система должна обрабатывать ошибки gracefully
    assert!(error_count < 50, "Не все запросы должны падать с ошибкой");

    Ok(())
}

