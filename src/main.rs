use anyhow::Result;
use log::info;
use std::process;

mod config;
mod wallet;
mod dex;
mod arbitrage;
mod monitor;
mod web;

use config::Config;
use monitor::Monitor;
use std::sync::Arc;

/// Точка входа в приложение
#[tokio::main]
async fn main() {
    // Инициализация логирования
    env_logger::Builder::from_default_env()
        .format_timestamp_secs()
        .format_level(true)
        .init();

    info!("=== Запуск арбитражного бота Solana ===");

    // Загрузка конфигурации
    let config = match Config::load() {
        Ok(cfg) => {
            info!("Конфигурация загружена успешно");
            if cfg.safety.simulation_mode {
                info!("⚠️  РЕЖИМ СИМУЛЯЦИИ АКТИВЕН - реальные транзакции не выполняются");
            } else {
                info!("⚠️  РЕЖИМ ПРОДАКШН - реальные транзакции будут выполняться");
            }
            cfg
        }
        Err(e) => {
            eprintln!("Ошибка загрузки конфигурации: {}", e);
            process::exit(1);
        }
    };

    // Инициализация монитора
    let monitor = Monitor::new(&config);

    // Инициализация кошелька
    let wallet = match wallet::Wallet::new(&config) {
        Ok(w) => {
            info!("Кошелёк инициализирован");
            Arc::new(w)
        }
        Err(e) => {
            eprintln!("Ошибка инициализации кошелька: {}", e);
            process::exit(1);
        }
    };

    // Инициализация DEX менеджера
    let dex_manager = match dex::DexManager::new(&config) {
        Ok(dm) => {
            info!("DEX менеджер инициализирован");
            dm
        }
        Err(e) => {
            eprintln!("Ошибка инициализации DEX менеджера: {}", e);
            process::exit(1);
        }
    };

    // Инициализация движка арбитража
    let arb_engine = arbitrage::ArbitrageEngine::new(
        config.clone(),
        wallet.clone(),
        dex_manager,
        monitor.clone(),
    );

    // Обёртка движка арбитража для совместного использования
    let arb_engine_shared = Arc::new(tokio::sync::Mutex::new(arb_engine));

    // Запуск веб-сервера (если включён)
    if config.web.enabled {
        let web_state = web::create_state(
            config.clone(),
            monitor.clone(),
            wallet.clone(),
            arb_engine_shared.clone(),
        );
        
        let web_config = config.clone();
        tokio::spawn(async move {
            if let Err(e) = web::start_server(web_state, &web_config).await {
                log::error!("Ошибка веб-сервера: {}", e);
            }
        });
        info!("Веб-сервер запущен на http://{}:{}", config.web.bind_address, config.web.port);
    }

    // Запуск основного цикла
    let arb_engine_for_loop = arb_engine_shared.clone();
    if let Err(e) = run_arbitrage_loop(arb_engine_for_loop, config, monitor).await {
        eprintln!("Критическая ошибка: {}", e);
        process::exit(1);
    }
}

/// Основной цикл поиска и выполнения арбитража
async fn run_arbitrage_loop(
    engine: Arc<tokio::sync::Mutex<arbitrage::ArbitrageEngine>>,
    config: Config,
    monitor: Monitor,
) -> Result<()> {
    let check_interval = std::time::Duration::from_millis(config.monitoring.check_interval_ms);

    loop {
        let opportunities = {
            let engine_guard = engine.lock().await;
            engine_guard.find_opportunities().await
        };

        match opportunities {
            Ok(opportunities) => {
                if opportunities.is_empty() {
                    log::debug!("Арбитражные возможности не найдены");
                } else {
                    log::info!("Найдено {} возможностей", opportunities.len());
                    for opp in opportunities {
                        log::info!("Возможность: {} -> {} (прибыль: {:.2}%, после комиссий: {:.2}%)", 
                            opp.from_dex, opp.to_dex, opp.profit_percent, opp.profit_percent_after_fees);
                        
                        let result = {
                            let mut engine_guard = engine.lock().await;
                            engine_guard.execute_arbitrage(opp).await
                        };
                        
                        match result {
                            Ok(_) => {
                                // Успешное выполнение - счётчик неудач уже сброшен в execute_arbitrage
                            }
                            Err(e) => {
                                log::error!("Ошибка выполнения арбитража: {}", e);
                                // Проверка лимита неудач (execute_arbitrage уже проверил, но на всякий случай)
                                // Если достигнут лимит, execute_arbitrage вернёт ошибку, которую нужно обработать
                            }
                        }
                    }
                }
            }
            Err(e) => {
                log::error!("Ошибка поиска возможностей: {}", e);
            }
        }

        tokio::time::sleep(check_interval).await;
    }
}

