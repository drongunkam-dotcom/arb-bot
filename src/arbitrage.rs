// arbitrage.rs — основной движок арбитража
// Назначение: поиск арбитражных возможностей и принятие решений о сделках

use anyhow::Result;
use rust_decimal::Decimal;
use std::sync::Arc;
use std::time::Duration;
use log::{info, warn, error};

use crate::config::Config;
use crate::wallet::Wallet;
use crate::dex::{DexClient, DexQuote};
use crate::monitor::Monitor;

pub struct ArbitrageEngine {
    config: Config,
    wallet: Arc<Wallet>,
    monitor: Arc<Monitor>,
    dex_client: DexClient,
}

impl ArbitrageEngine {
    pub fn new(config: Config, wallet: Arc<Wallet>, monitor: Arc<Monitor>) -> Self {
        let dex_client = DexClient::new(&config);
        
        Self {
            config,
            wallet,
            monitor,
            dex_client,
        }
    }
    
    /// Основной цикл арбитража
    pub async fn run(&self) -> Result<()> {
        loop {
            // 1. Получаем котировки со всех DEX
            let all_quotes = match self.dex_client.fetch_all_quotes().await {
                Ok(q) => q,
                Err(e) => {
                    let stop = self.monitor.log_failure("fetch_all_quotes", &e.to_string());
                    if stop {
                        return Err(anyhow::anyhow!("Слишком много ошибок при получении котировок"));
                    }
                    // Ждём и пробуем снова
                    tokio::time::sleep(Duration::from_millis(
                        self.config.monitoring.check_interval_ms
                    )).await;
                    continue;
                }
            };
            
            // 2. Ищем арбитражные возможности
            let opportunities = self.find_arbitrage_opportunities(&all_quotes)?;
            
            if opportunities.is_empty() {
                self.monitor.log_info("Арбитражных возможностей не найдено в этом цикле");
            } else {
                info!("Найдено {} потенциальных возможностей арбитража", opportunities.len());
            }
            
            // 3. Обрабатываем найденные возможности
            for opp in opportunities {
                let profit_percent = opp.profit_percent;
                
                if profit_percent < self.config.arbitrage.min_profit_percent {
                    continue;
                }
                
                info!(
                    "💰 Обнаружена возможность: {} -> {} по паре {} (профит: {:.4}%)",
                    opp.buy_dex, opp.sell_dex, opp.pair, profit_percent
                );
                
                if self.config.safety.simulation_mode {
                    info!("SIMULATION_MODE=ON — сделка НЕ будет выполнена (только лог)");
                    self.monitor.log_success(profit_percent);
                } else {
                    // ⚠️ Здесь будет реальное выполнение транзакции
                    warn!("ПРОДАКШН РЕЖИМ: в этой точке должен выполняться реальный трейд");
                    // TODO: реализовать создание и отправку транзакций
                    self.monitor.log_success(profit_percent);
                }
            }
            
            // 4. Ждём до следующей итерации
            tokio::time::sleep(Duration::from_millis(
                self.config.monitoring.check_interval_ms
            )).await;
        }
    }
    
    /// Структура для описания арбитражной возможности
    pub struct ArbitrageOpportunity {
        pub pair: String,
        pub buy_dex: String,
        pub sell_dex: String,
        pub profit_percent: f64,
    }
    
    /// Поиск арбитражных возможностей на основе котировок
    fn find_arbitrage_opportunities(
        &self,
        all_quotes: &std::collections::HashMap<String, Vec<DexQuote>>,
    ) -> Result<Vec<ArbitrageOpportunity>> {
        let mut result = Vec::new();
        
        // Простейший алгоритм:
        // Для каждой пары токенов находим:
        // - DEX, где можно купить дешевле всего
        // - DEX, где можно продать дороже всего
        // И считаем потенциальный профит.
        
        // Собираем по парам все котировки
        use std::collections::HashMap;
        let mut by_pair: HashMap<String, Vec<&DexQuote>> = HashMap::new();
        
        for (_dex_name, quotes) in all_quotes {
            for quote in quotes {
                by_pair.entry(quote.pair.clone())
                    .or_default()
                    .push(quote);
            }
        }
        
        for (pair, quotes) in by_pair {
            if quotes.len() < 2 {
                continue;
            }
            
            // Ищем минимум и максимум по цене
            let mut maybe_min: Option<&DexQuote> = None;
            let mut maybe_max: Option<&DexQuote> = None;
            
            for q in &quotes {
                if maybe_min.is_none() || q.price < maybe_min.unwrap().price {
                    maybe_min = Some(*q);
                }
                if maybe_max.is_none() || q.price > maybe_max.unwrap().price {
                    maybe_max = Some(*q);
                }
            }
            
            if let (Some(min_q), Some(max_q)) = (maybe_min, maybe_max) {
                if max_q.price <= min_q.price {
                    continue;
                }
                
                let profit = (max_q.price - min_q.price) / min_q.price * Decimal::new(100, 0);
                let profit_f64: f64 = profit.try_into().unwrap_or(0.0);
                
                if profit_f64 > 0.0 {
                    result.push(ArbitrageOpportunity {
                        pair: pair.clone(),
                        buy_dex: min_q.dex_name.clone(),
                        sell_dex: max_q.dex_name.clone(),
                        profit_percent: profit_f64,
                    });
                }
            }
        }
        
        Ok(result)
    }
}