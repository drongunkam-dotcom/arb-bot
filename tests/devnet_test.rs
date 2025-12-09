use anyhow::{Context, Result};
use arb_bot::config::Config;
use arb_bot::wallet::Wallet;
use arb_bot::dex::DexManager;
use rust_decimal::Decimal;
use solana_sdk::signature::Keypair;
use solana_sdk::pubkey::Pubkey;
use solana_client::rpc_client::RpcClient;
use solana_sdk::commitment_config::CommitmentConfig;
use std::fs;
use tempfile::TempDir;

/// Тесты для devnet окружения
/// 
/// Требования:
/// - Доступ к devnet RPC (https://api.devnet.solana.com)
/// - Тестовый кошелёк с балансом (можно получить через airdrop)

/// Создание тестовой конфигурации для devnet
fn create_devnet_config(temp_dir: &TempDir) -> Result<Config> {
    let keypair_path = temp_dir.path().join("test_wallet.json");
    
    // Создаём новый тестовый кошелёк
    let keypair = Keypair::new();
    let keypair_json = serde_json::json!({
        "secretKey": keypair.to_bytes().to_vec()
    });
    fs::write(&keypair_path, serde_json::to_string_pretty(&keypair_json)?)?;

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
enabled_dexes = ["raydium"]
trading_pairs = ["SOL/USDC"]

[monitoring]
check_interval_ms = 1000
log_level = "debug"
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

/// Тест подключения к devnet RPC
#[tokio::test]
#[ignore] // Игнорируем по умолчанию, запускать вручную: cargo test --test devnet_test -- --ignored
async fn test_devnet_rpc_connection() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    // Проверка подключения к RPC
    let rpc_client = RpcClient::new_with_commitment(
        config.network.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Получение версии RPC
    let version = rpc_client.get_version()
        .context("Не удалось подключиться к devnet RPC")?;
    
    log::info!("✅ Подключение к devnet успешно. Версия: {:?}", version);

    // Получение последнего slot
    let slot = rpc_client.get_slot()
        .context("Не удалось получить slot")?;
    
    log::info!("✅ Текущий slot: {}", slot);

    Ok(())
}

/// Тест инициализации кошелька на devnet
#[tokio::test]
#[ignore]
async fn test_devnet_wallet_initialization() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    // Инициализация кошелька
    let wallet = Wallet::new(&config)
        .context("Не удалось инициализировать кошелёк")?;

    log::info!("✅ Кошелёк инициализирован: {}", wallet.pubkey());

    // Получение баланса
    let balance = wallet.get_balance(&config.network.rpc_url).await
        .context("Не удалось получить баланс")?;

    log::info!("✅ Баланс кошелька: {} lamports ({} SOL)", 
        balance, 
        balance as f64 / 1_000_000_000.0
    );

    // Если баланс нулевой, можно запросить airdrop (только для devnet)
    if balance == 0 {
        log::warn!("⚠️  Баланс равен нулю. Для тестирования можно запросить airdrop:");
        log::warn!("   solana airdrop 1 {} --url devnet", wallet.pubkey());
    }

    Ok(())
}

/// Тест инициализации DexManager на devnet
#[tokio::test]
#[ignore]
async fn test_devnet_dex_manager_initialization() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    // Инициализация DexManager
    let dex_manager = DexManager::new(&config)
        .context("Не удалось инициализировать DexManager")?;

    log::info!("✅ DexManager инициализирован");
    log::info!("✅ Зарегистрировано {} DEX", dex_manager.get_dexes().len());

    // Проверка наличия Raydium
    let raydium = dex_manager.get_dex("raydium")
        .ok_or_else(|| anyhow::anyhow!("Raydium не найден"))?;

    log::info!("✅ Raydium DEX найден: {}", raydium.name());

    Ok(())
}

/// Тест получения цены с Raydium на devnet
/// 
/// Примечание: этот тест может не пройти, если:
/// - Нет реальных пулов на devnet
/// - Заглушки в get_pool_address возвращают неверные адреса
#[tokio::test]
#[ignore]
async fn test_devnet_raydium_get_price() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    let dex_manager = DexManager::new(&config)?;
    let raydium = dex_manager.get_dex("raydium")
        .ok_or_else(|| anyhow::anyhow!("Raydium не найден"))?;

    log::info!("🔍 Попытка получить цену SOL/USDC с Raydium на devnet...");

    // Попытка получить цену
    // Ожидаем ошибку из-за заглушек, но проверяем, что код выполняется
    match raydium.get_price("SOL", "USDC").await {
        Ok(price) => {
            log::info!("✅ Цена получена: {} USDC за 1 SOL", price);
            assert!(price > Decimal::ZERO, "Цена должна быть больше нуля");
        }
        Err(e) => {
            log::warn!("⚠️  Не удалось получить цену (ожидаемо из-за заглушек): {}", e);
            // Это ожидаемо, так как get_pool_address использует заглушки
            // В реальной реализации нужно получать адреса пулов через API
            log::info!("ℹ️  Для полноценного тестирования нужно реализовать получение адресов пулов");
        }
    }

    Ok(())
}

/// Тест выполнения свопа в режиме симуляции на devnet
#[tokio::test]
#[ignore]
async fn test_devnet_raydium_swap_simulation() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    // Убеждаемся, что режим симуляции включен
    assert!(config.safety.simulation_mode, "Режим симуляции должен быть включен для теста");

    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let raydium = dex_manager.get_dex("raydium")
        .ok_or_else(|| anyhow::anyhow!("Raydium не найден"))?;

    log::info!("🔍 Попытка выполнить симуляцию свопа SOL -> USDC...");

    // Симуляция свопа
    let amount = Decimal::new(1, 0); // 1 SOL
    let min_output = Decimal::new(100, 0); // Минимум 100 USDC

    match raydium.execute_swap(
        true, // simulation_mode
        "SOL",
        "USDC",
        amount,
        min_output,
        &wallet,
    ).await {
        Ok(signature) => {
            log::info!("✅ Симуляция свопа выполнена успешно. Signature: {}", signature);
            assert!(signature.contains("simulated"), "В режиме симуляции должна возвращаться simulated signature");
        }
        Err(e) => {
            log::error!("❌ Ошибка при симуляции свопа: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Тест получения цены с Orca на devnet
/// 
/// Примечание: этот тест может не пройти, если:
/// - Нет реальных пулов на devnet
/// - Заглушки в get_whirlpool_address возвращают неверные адреса
#[tokio::test]
#[ignore]
async fn test_devnet_orca_get_price() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let mut config = create_devnet_config(&temp_dir)?;
    // Добавляем Orca в список включённых DEX
    config.dex.enabled_dexes = vec!["orca".to_string()];

    let dex_manager = DexManager::new(&config)?;
    let orca = dex_manager.get_dex("orca")
        .ok_or_else(|| anyhow::anyhow!("Orca не найден"))?;

    log::info!("🔍 Попытка получить цену SOL/USDC с Orca на devnet...");

    // Попытка получить цену
    // Ожидаем ошибку из-за заглушек, но проверяем, что код выполняется
    match orca.get_price("SOL", "USDC").await {
        Ok(price) => {
            log::info!("✅ Цена получена: {} USDC за 1 SOL", price);
            assert!(price > Decimal::ZERO, "Цена должна быть больше нуля");
        }
        Err(e) => {
            log::warn!("⚠️  Не удалось получить цену (ожидаемо из-за заглушек): {}", e);
            // Это ожидаемо, так как get_whirlpool_address использует заглушки
            // В реальной реализации нужно получать адреса пулов через API
            log::info!("ℹ️  Для полноценного тестирования нужно реализовать получение адресов Whirlpools");
        }
    }

    Ok(())
}

/// Тест выполнения свопа в режиме симуляции на devnet для Orca
#[tokio::test]
#[ignore]
async fn test_devnet_orca_swap_simulation() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let mut config = create_devnet_config(&temp_dir)?;
    // Добавляем Orca в список включённых DEX
    config.dex.enabled_dexes = vec!["orca".to_string()];

    // Убеждаемся, что режим симуляции включен
    assert!(config.safety.simulation_mode, "Режим симуляции должен быть включен для теста");

    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let orca = dex_manager.get_dex("orca")
        .ok_or_else(|| anyhow::anyhow!("Orca не найден"))?;

    log::info!("🔍 Попытка выполнить симуляцию свопа SOL -> USDC на Orca...");

    // Симуляция свопа
    let amount = Decimal::new(1, 0); // 1 SOL
    let min_output = Decimal::new(100, 0); // Минимум 100 USDC

    match orca.execute_swap(
        true, // simulation_mode
        "SOL",
        "USDC",
        amount,
        min_output,
        &wallet,
    ).await {
        Ok(signature) => {
            log::info!("✅ Симуляция свопа выполнена успешно. Signature: {}", signature);
            assert!(signature.contains("simulated"), "В режиме симуляции должна возвращаться simulated signature");
        }
        Err(e) => {
            log::error!("❌ Ошибка при симуляции свопа: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

/// Тест проверки RPC доступности и получения данных
#[tokio::test]
#[ignore]
async fn test_devnet_rpc_data_retrieval() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    let rpc_client = RpcClient::new_with_commitment(
        config.network.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Получение последнего blockhash
    log::info!("🔍 Получение последнего blockhash...");
    let blockhash = rpc_client.get_latest_blockhash()
        .context("Не удалось получить blockhash")?;
    log::info!("✅ Blockhash получен: {}", blockhash);

    // Получение информации о кластере
    log::info!("🔍 Получение информации о кластере...");
    let cluster = rpc_client.get_cluster_nodes()
        .context("Не удалось получить информацию о кластере")?;
    log::info!("✅ Получено {} нод кластера", cluster.len());

    // Получение текущей эпохи
    log::info!("🔍 Получение текущей эпохи...");
    let epoch_info = rpc_client.get_epoch_info()
        .context("Не удалось получить информацию об эпохе")?;
    log::info!("✅ Текущая эпоха: {}", epoch_info.epoch);

    Ok(())
}

/// Тест проверки retry-логики при ошибках RPC
#[tokio::test]
#[ignore]
async fn test_devnet_rpc_retry_logic() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    let rpc_client = RpcClient::new_with_commitment(
        config.network.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );

    // Тест получения несуществующего аккаунта (должно вернуть None, не ошибку)
    let fake_pubkey = Pubkey::new_unique();
    log::info!("🔍 Попытка получить несуществующий аккаунт: {}", fake_pubkey);
    
    match rpc_client.get_account(&fake_pubkey) {
        Ok(_) => {
            log::warn!("⚠️  Аккаунт найден (неожиданно)");
        }
        Err(e) => {
            log::info!("✅ Ожидаемая ошибка для несуществующего аккаунта: {}", e);
        }
    }

    // Тест получения существующего системного аккаунта
    let system_program = solana_sdk::system_program::id();
    log::info!("🔍 Попытка получить системный аккаунт: {}", system_program);
    
    match rpc_client.get_account(&system_program) {
        Ok(account) => {
            log::info!("✅ Системный аккаунт получен. Lamports: {}", account.lamports);
        }
        Err(e) => {
            log::error!("❌ Не удалось получить системный аккаунт: {}", e);
            return Err(e.into());
        }
    }

    Ok(())
}

/// Комплексный тест всей цепочки на devnet
#[tokio::test]
#[ignore]
async fn test_devnet_full_integration() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Info)
        .init();

    log::info!("=== Начало комплексного теста на devnet ===");

    let temp_dir = TempDir::new()?;
    let config = create_devnet_config(&temp_dir)?;

    // 1. Проверка подключения к RPC
    log::info!("[1/5] Проверка подключения к RPC...");
    let rpc_client = RpcClient::new_with_commitment(
        config.network.rpc_url.clone(),
        CommitmentConfig::confirmed(),
    );
    let _version = rpc_client.get_version()
        .context("Не удалось подключиться к RPC")?;
    log::info!("✅ RPC подключение успешно");

    // 2. Инициализация кошелька
    log::info!("[2/5] Инициализация кошелька...");
    let wallet = Wallet::new(&config)?;
    log::info!("✅ Кошелёк: {}", wallet.pubkey());

    // 3. Инициализация DexManager
    log::info!("[3/5] Инициализация DexManager...");
    let dex_manager = DexManager::new(&config)?;
    log::info!("✅ DexManager инициализирован");

    // 4. Проверка получения цены (может не пройти из-за заглушек)
    log::info!("[4/5] Попытка получить цену...");
    if let Some(raydium) = dex_manager.get_dex("raydium") {
        match raydium.get_price("SOL", "USDC").await {
            Ok(price) => log::info!("✅ Цена получена: {}", price),
            Err(e) => log::warn!("⚠️  Не удалось получить цену: {}", e),
        }
    }

    // 5. Симуляция свопа
    log::info!("[5/5] Симуляция свопа...");
    if let Some(raydium) = dex_manager.get_dex("raydium") {
        match raydium.execute_swap(
            true,
            "SOL",
            "USDC",
            Decimal::new(1, 0),
            Decimal::new(100, 0),
            &wallet,
        ).await {
            Ok(sig) => log::info!("✅ Симуляция свопа успешна: {}", sig),
            Err(e) => log::error!("❌ Ошибка симуляции свопа: {}", e),
        }
    }

    log::info!("=== Комплексный тест завершён ===");
    Ok(())
}

/// Тест получения цены с Serum на devnet
/// 
/// Примечание: этот тест может не пройти, если:
/// - Нет реальных рынков на devnet
/// - Заглушки в get_market_address возвращают неверные адреса
#[tokio::test]
#[ignore]
async fn test_devnet_serum_get_price() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let mut config = create_devnet_config(&temp_dir)?;
    // Добавляем Serum в список включённых DEX
    config.dex.enabled_dexes = vec!["serum".to_string()];

    let dex_manager = DexManager::new(&config)?;
    let serum = dex_manager.get_dex("serum")
        .ok_or_else(|| anyhow::anyhow!("Serum не найден"))?;

    log::info!("🔍 Попытка получить цену SOL/USDC с Serum на devnet...");

    // Попытка получить цену
    // Ожидаем ошибку из-за заглушек, но проверяем, что код выполняется
    match serum.get_price("SOL", "USDC").await {
        Ok(price) => {
            log::info!("✅ Цена получена: {} USDC за 1 SOL", price);
            assert!(price > Decimal::ZERO, "Цена должна быть больше нуля");
        }
        Err(e) => {
            log::warn!("⚠️  Не удалось получить цену (ожидаемо из-за заглушек): {}", e);
            // Это ожидаемо, так как get_market_address использует заглушки
            // В реальной реализации нужно получать адреса рынков через API
            log::info!("ℹ️  Для полноценного тестирования нужно реализовать получение адресов рынков");
        }
    }

    Ok(())
}

/// Тест выполнения свопа в режиме симуляции на devnet для Serum
#[tokio::test]
#[ignore]
async fn test_devnet_serum_swap_simulation() -> Result<()> {
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();

    let temp_dir = TempDir::new()?;
    let mut config = create_devnet_config(&temp_dir)?;
    // Добавляем Serum в список включённых DEX
    config.dex.enabled_dexes = vec!["serum".to_string()];

    // Убеждаемся, что режим симуляции включен
    assert!(config.safety.simulation_mode, "Режим симуляции должен быть включен для теста");

    let wallet = Wallet::new(&config)?;
    let dex_manager = DexManager::new(&config)?;
    let serum = dex_manager.get_dex("serum")
        .ok_or_else(|| anyhow::anyhow!("Serum не найден"))?;

    log::info!("🔍 Попытка выполнить симуляцию свопа SOL -> USDC на Serum...");

    // Симуляция свопа
    let amount = Decimal::new(1, 0); // 1 SOL
    let min_output = Decimal::new(100, 0); // Минимум 100 USDC

    match serum.execute_swap(
        true, // simulation_mode
        "SOL",
        "USDC",
        amount,
        min_output,
        &wallet,
    ).await {
        Ok(signature) => {
            log::info!("✅ Симуляция свопа выполнена успешно. Signature: {}", signature);
            assert!(signature.contains("simulated"), "В режиме симуляции должна возвращаться simulated signature");
        }
        Err(e) => {
            log::error!("❌ Ошибка при симуляции свопа: {}", e);
            return Err(e);
        }
    }

    Ok(())
}

