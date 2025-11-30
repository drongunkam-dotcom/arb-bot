// main.rs — точка входа арбитражного бота
// Назначение: инициализация, запуск основного цикла, обработка сигналов завершения

use anyhow::Result;
use log::{info, error};
use std::sync::Arc;
use tokio::signal;

mod config;
mod wallet;
mod dex;
mod arbitrage;
mod monitor;

use config::Config;
use wallet::Wallet;
use arbitrage::ArbitrageEngine;
use monitor::Monitor;

#[tokio::main]
async fn main() -> Result<()> {
    // Инициализация логирования
    env_logger::init();
    
    info!("=== Запуск арбитражного бота Solana ===");
    
    // Загрузка конфигурации из config.toml и .env
    let config = Config::load()?;
    info!("Конфигурация загружена успешно");
    
    // Проверка режима симуляции
    if config.safety.simulation_mode {
        info!("⚠️  РЕЖИМ СИМУЛЯЦИИ АКТИВЕН — реальные транзакции не выполняются");
    } else {
        info!("🔴 ПРОДАКШН РЕЖИМ — будут выполняться реальные транзакции!");
    }
    
    // Инициализация кошелька
    let wallet = Arc::new(Wallet::new(&config)?);
    info!("Кошелёк инициализирован: {}", wallet.public_key());
    
    // Проверка баланса
    let balance = wallet.get_balance().await?;
    info!("Текущий баланс: {} SOL", balance);
    
    if balance < config.safety.min_balance_sol {
        error!(
            "Недостаточный баланс! Минимум: {} SOL",
            config.safety.min_balance_sol
        );
        return Err(anyhow::anyhow!("Недостаточный баланс"));
    }
    
    // Инициализация монитора
    let monitor = Arc::new(Monitor::new(&config));
    
    // Инициализация движка арбитража
    let engine = ArbitrageEngine::new(config.clone(), wallet.clone(), monitor.clone());
    
    info!("Все компоненты инициализированы, запуск основного цикла...");
    
    // Запуск основного цикла с обработкой Ctrl+C
    tokio::select! {
        result = engine.run() => {
            match result {
                Ok(_) => info!("Движок завершил работу штатно"),
                Err(e) => error!("Ошибка в движке: {}", e),
            }
        }
        _ = signal::ctrl_c() => {
            info!("Получен сигнал завершения (Ctrl+C)");
        }
    }
    
    info!("=== Завершение работы бота ===");
    Ok(())
}