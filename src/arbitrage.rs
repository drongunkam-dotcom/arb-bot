use anyhow::{Context, Result};
use rust_decimal::Decimal;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::timeout;
use crate::config::Config;
use crate::wallet::Wallet;
use crate::dex::{DexManager, DexInterface};
use crate::monitor::Monitor;
use std::sync::Arc;

/// Арбитражная возможность
#[derive(Debug, Clone)]
pub struct ArbitrageOpportunity {
    pub from_dex: String,
    pub to_dex: String,
    pub base_token: String,
    pub quote_token: String,
    pub buy_price: Decimal,
    pub sell_price: Decimal,
    pub profit_percent: Decimal,
    pub profit_percent_after_fees: Decimal, // Прибыль с учётом комиссий
    pub trade_amount: Decimal,
    pub estimated_fees: Decimal, // Оценка комиссий
}

/// Движок арбитража
pub struct ArbitrageEngine {
    config: Config,
    wallet: Arc<Wallet>,
    dex_manager: DexManager,
    monitor: Monitor,
    consecutive_failures: u32,
}

impl ArbitrageEngine {
    /// Создание нового движка арбитража
    pub fn new(
        config: Config,
        wallet: Arc<Wallet>,
        dex_manager: DexManager,
        monitor: Monitor,
    ) -> Self {
        Self {
            config,
            wallet,
            dex_manager,
            monitor,
            consecutive_failures: 0,
        }
    }

    /// Поиск арбитражных возможностей
    pub async fn find_opportunities(&self) -> Result<Vec<ArbitrageOpportunity>> {
        let mut opportunities = Vec::new();

        // Получение всех DEX
        let dexes = self.dex_manager.get_dexes();
        if dexes.len() < 2 {
            return Ok(opportunities); // Нужно минимум 2 DEX для арбитража
        }

        // Проверка каждой торговой пары
        for pair in &self.config.dex.trading_pairs {
            let parts: Vec<&str> = pair.split('/').collect();
            if parts.len() != 2 {
                log::warn!("Некорректный формат торговой пары: {}", pair);
                continue;
            }

            let base_token = parts[0];
            let quote_token = parts[1];

            // Получение цен со всех DEX
            let mut prices = Vec::new();
            for dex in dexes {
                match dex.get_price(base_token, quote_token).await {
                    Ok(price) => {
                        prices.push((dex.name(), price));
                    }
                    Err(e) => {
                        log::debug!("Ошибка получения цены с {}: {}", dex.name(), e);
                    }
                }
            }

            if prices.len() < 2 {
                continue; // Нужно минимум 2 цены для сравнения
            }

            // Поиск максимальной разницы в ценах
            for i in 0..prices.len() {
                for j in 0..prices.len() {
                    if i == j {
                        continue;
                    }

                    let (buy_dex, buy_price) = &prices[i];
                    let (sell_dex, sell_price) = &prices[j];

                    // Проверка возможности арбитража (покупка дешевле, продажа дороже)
                    if sell_price > buy_price {
                        let profit_percent = ((sell_price - buy_price) / buy_price) * Decimal::from(100);

                        // Расчёт оптимального объёма сделки (до учёта комиссий)
                        let trade_amount = self.calculate_optimal_trade_amount(
                            *buy_price,
                            *sell_price,
                            base_token,
                            quote_token,
                            buy_dex,
                            sell_dex,
                        ).await?;

                        // Получение комиссий DEX
                        let buy_fee_percent = self.get_dex_fee(buy_dex).await.unwrap_or(Decimal::from_str("0.25")?); // 0.25% по умолчанию
                        let sell_fee_percent = self.get_dex_fee(sell_dex).await.unwrap_or(Decimal::from_str("0.25")?); // 0.25% по умолчанию
                        let total_fee_percent = buy_fee_percent + sell_fee_percent;

                        // Расчёт прибыли с учётом комиссий
                        let profit_after_fees = profit_percent - total_fee_percent;
                        
                        // Оценка комиссий в SOL
                        let estimated_fees = trade_amount * (total_fee_percent / Decimal::from(100));

                        let min_profit = Decimal::from_str(&format!("{:.10}", self.config.arbitrage.min_profit_percent))
                            .unwrap_or(Decimal::ZERO);
                        
                        // Проверка минимальной прибыли с учётом комиссий
                        if profit_after_fees >= min_profit {
                            opportunities.push(ArbitrageOpportunity {
                                from_dex: buy_dex.to_string(),
                                to_dex: sell_dex.to_string(),
                                base_token: base_token.to_string(),
                                quote_token: quote_token.to_string(),
                                buy_price: *buy_price,
                                sell_price: *sell_price,
                                profit_percent,
                                profit_percent_after_fees: profit_after_fees,
                                trade_amount,
                                estimated_fees,
                            });
                        }
                    }
                }
            }
        }

        // Сортировка по прибыльности с учётом комиссий
        opportunities.sort_by(|a, b| b.profit_percent_after_fees.cmp(&a.profit_percent_after_fees));

        Ok(opportunities)
    }

    /// Выполнение арбитража
    pub async fn execute_arbitrage(&mut self, opportunity: ArbitrageOpportunity) -> Result<()> {
        let simulation_mode = self.config.safety.simulation_mode;

        log::info!(
            "Выполнение арбитража: {} -> {} (прибыль: {:.2}%, после комиссий: {:.2}%)",
            opportunity.from_dex,
            opportunity.to_dex,
            opportunity.profit_percent,
            opportunity.profit_percent_after_fees
        );

        // Получение DEX
        let buy_dex = self.dex_manager.get_dex(&opportunity.from_dex)
            .ok_or_else(|| anyhow::anyhow!("DEX не найден: {}", opportunity.from_dex))?;
        let sell_dex = self.dex_manager.get_dex(&opportunity.to_dex)
            .ok_or_else(|| anyhow::anyhow!("DEX не найден: {}", opportunity.to_dex))?;

        // Получение актуального slippage из пулов
        let actual_slippage = self.get_actual_slippage(
            buy_dex,
            sell_dex,
            &opportunity.base_token,
            &opportunity.quote_token,
            opportunity.trade_amount,
        ).await.unwrap_or_else(|e| {
            log::warn!("Не удалось получить актуальный slippage, используем значение из конфига: {}", e);
            Decimal::from_str(&format!("{:.10}", self.config.arbitrage.slippage_tolerance))
                .unwrap_or(Decimal::from_str("1.0").unwrap()) // 1% по умолчанию
        });

        // Расчёт минимального выхода с учётом актуального slippage
        let slippage_multiplier = Decimal::from(1) - (actual_slippage / Decimal::from(100));
        let min_output = opportunity.trade_amount * opportunity.sell_price * slippage_multiplier;

        // Таймаут для транзакций
        let tx_timeout = Duration::from_secs(self.config.arbitrage.transaction_timeout_sec);

        // Попытка атомарного выполнения (если возможно)
        let result = if self.can_execute_atomically(buy_dex, sell_dex) {
            self.execute_atomic_arbitrage(
                buy_dex,
                sell_dex,
                &opportunity,
                min_output,
                simulation_mode,
                tx_timeout,
            ).await
        } else {
            // Выполнение в два этапа
            self.execute_two_step_arbitrage(
                buy_dex,
                sell_dex,
                &opportunity,
                min_output,
                simulation_mode,
                tx_timeout,
            ).await
        };

        match result {
            Ok((buy_sig, sell_sig)) => {
                log::info!("Покупка выполнена: {}", buy_sig);
                log::info!("Продажа выполнена: {}", sell_sig);
                
                // Обновление статистики при успехе
                self.consecutive_failures = 0;
                
                if simulation_mode {
                    log::info!("✅ Арбитраж выполнен (симуляция): прибыль {:.2}% (после комиссий: {:.2}%)", 
                        opportunity.profit_percent, opportunity.profit_percent_after_fees);
                } else {
                    log::info!("✅ Арбитраж выполнен (реально): прибыль {:.2}% (после комиссий: {:.2}%)", 
                        opportunity.profit_percent, opportunity.profit_percent_after_fees);
                }

                // Логирование через монитор
                self.monitor.log_arbitrage(
                    &opportunity.from_dex,
                    &opportunity.to_dex,
                    opportunity.profit_percent_after_fees,
                    simulation_mode,
                );

                Ok(())
            }
            Err(e) => {
                // Увеличение счётчика неудач
                self.consecutive_failures += 1;
                log::error!("Ошибка выполнения арбитража (неудач подряд: {}): {}", 
                    self.consecutive_failures, e);
                
                // Проверка лимита неудач
                if self.consecutive_failures >= self.config.safety.max_consecutive_failures {
                    anyhow::bail!(
                        "Достигнут лимит последовательных неудач ({}), остановка выполнения",
                        self.config.safety.max_consecutive_failures
                    );
                }
                
                Err(e)
            }
        }
    }

    /// Расчёт оптимального объёма сделки с учётом ликвидности и комиссий
    async fn calculate_optimal_trade_amount(
        &self,
        _buy_price: Decimal,
        _sell_price: Decimal,
        base_token: &str,
        quote_token: &str,
        buy_dex: &str,
        sell_dex: &str,
    ) -> Result<Decimal> {
        let max_amount = Decimal::from_str(&format!("{:.10}", self.config.arbitrage.max_trade_amount_sol))
            .unwrap_or(Decimal::ZERO);

        // Получение доступной ликвидности на DEX
        let buy_liquidity = self.get_dex_liquidity(buy_dex, base_token, quote_token).await
            .unwrap_or(max_amount * Decimal::from(10)); // Если не удалось получить, предполагаем достаточную ликвидность
        let sell_liquidity = self.get_dex_liquidity(sell_dex, base_token, quote_token).await
            .unwrap_or(max_amount * Decimal::from(10));

        // Используем минимальную ликвидность из двух DEX
        let available_liquidity = buy_liquidity.min(sell_liquidity);

        // Расчёт оптимального объёма: используем 10% от доступной ликвидности или max_amount, что меньше
        // Это помогает избежать большого price impact
        let optimal_from_liquidity = available_liquidity * Decimal::from_str("0.1")?;
        
        // Выбираем минимум из: max_amount, optimal_from_liquidity, и доступной ликвидности
        let optimal_amount = max_amount.min(optimal_from_liquidity).min(available_liquidity);

        // Проверка, что объём больше нуля
        if optimal_amount <= Decimal::ZERO {
            anyhow::bail!("Недостаточная ликвидность для выполнения арбитража");
        }

        log::debug!(
            "Расчёт объёма: max={}, buy_liq={}, sell_liq={}, optimal={}",
            max_amount, buy_liquidity, sell_liquidity, optimal_amount
        );

        Ok(optimal_amount)
    }

    /// Получение комиссии DEX в процентах
    async fn get_dex_fee(&self, dex_name: &str) -> Result<Decimal> {
        // В реальной реализации нужно получать комиссию из DEX API или конфигурации
        // Для разных DEX комиссии разные:
        // Raydium: обычно 0.25%
        // Orca: обычно 0.3%
        // Serum: обычно 0.04%
        let fee = match dex_name {
            "raydium" => Decimal::from_str("0.25")?,
            "orca" => Decimal::from_str("0.3")?,
            "serum" => Decimal::from_str("0.04")?,
            _ => Decimal::from_str("0.25")?, // По умолчанию
        };
        Ok(fee)
    }

    /// Получение доступной ликвидности на DEX
    async fn get_dex_liquidity(&self, dex_name: &str, base_token: &str, quote_token: &str) -> Result<Decimal> {
        // В реальной реализации нужно получать ликвидность из пула
        // Для упрощения возвращаем большое значение
        // TODO: Реализовать получение реальной ликвидности из пулов
        let dex = self.dex_manager.get_dex(dex_name)
            .ok_or_else(|| anyhow::anyhow!("DEX не найден: {}", dex_name))?;
        
        // Пытаемся получить цену, чтобы проверить доступность пула
        match dex.get_price(base_token, quote_token).await {
            Ok(_) => {
                // Если цена получена, предполагаем достаточную ликвидность
                Ok(Decimal::from_str("1000")?) // 1000 SOL по умолчанию
            }
            Err(_) => {
                // Если не удалось получить цену, ликвидность нулевая
                Ok(Decimal::ZERO)
            }
        }
    }

    /// Получение актуального slippage из пулов
    async fn get_actual_slippage(
        &self,
        _buy_dex: &dyn DexInterface,
        _sell_dex: &dyn DexInterface,
        _base_token: &str,
        _quote_token: &str,
        _trade_amount: Decimal,
    ) -> Result<Decimal> {
        // В реальной реализации нужно симулировать своп и получить реальный slippage
        // Для упрощения используем значение из конфигурации
        // TODO: Реализовать симуляцию свопа для получения актуального slippage
        Ok(Decimal::from_str(&format!("{:.10}", self.config.arbitrage.slippage_tolerance))
            .unwrap_or(Decimal::from_str("1.0")?))
    }

    /// Проверка возможности атомарного выполнения
    fn can_execute_atomically(&self, _buy_dex: &dyn DexInterface, _sell_dex: &dyn DexInterface) -> bool {
        // Атомарное выполнение возможно только если оба свопа можно объединить в одну транзакцию
        // На Solana это возможно, если оба DEX поддерживают это
        // Для упрощения возвращаем false, так как требуется дополнительная реализация
        // TODO: Реализовать проверку поддержки атомарных транзакций
        false
    }

    /// Выполнение атомарного арбитража (покупка и продажа в одной транзакции)
    async fn execute_atomic_arbitrage(
        &self,
        buy_dex: &dyn DexInterface,
        sell_dex: &dyn DexInterface,
        opportunity: &ArbitrageOpportunity,
        min_output: Decimal,
        simulation_mode: bool,
        tx_timeout: Duration,
    ) -> Result<(String, String)> {
        // Атомарное выполнение требует объединения инструкций от обоих DEX в одну транзакцию
        // Это сложная реализация, требующая доступа к внутренним методам DEX
        // Для упрощения выполняем как две отдельные транзакции, но с таймаутом
        log::info!("Атомарное выполнение не реализовано, используем двухэтапное");
        self.execute_two_step_arbitrage(
            buy_dex,
            sell_dex,
            opportunity,
            min_output,
            simulation_mode,
            tx_timeout,
        ).await
    }

    /// Выполнение двухэтапного арбитража (покупка, затем продажа)
    async fn execute_two_step_arbitrage(
        &self,
        buy_dex: &dyn DexInterface,
        sell_dex: &dyn DexInterface,
        opportunity: &ArbitrageOpportunity,
        min_output: Decimal,
        simulation_mode: bool,
        tx_timeout: Duration,
    ) -> Result<(String, String)> {
        // Шаг 1: Покупка на первом DEX с таймаутом
        let buy_future = buy_dex.execute_swap(
            simulation_mode,
            &opportunity.quote_token,
            &opportunity.base_token,
            opportunity.trade_amount,
            Decimal::ZERO, // Минимальный выход для покупки
            &self.wallet,
        );

        let buy_signature = timeout(tx_timeout, buy_future)
            .await
            .context("Таймаут при выполнении покупки")?
            .context("Ошибка выполнения покупки")?;

        log::info!("Покупка выполнена: {}", buy_signature);

        // Небольшая задержка между транзакциями для подтверждения
        if !simulation_mode {
            tokio::time::sleep(Duration::from_millis(500)).await;
        }

        // Шаг 2: Продажа на втором DEX с таймаутом
        let sell_future = sell_dex.execute_swap(
            simulation_mode,
            &opportunity.base_token,
            &opportunity.quote_token,
            opportunity.trade_amount,
            min_output,
            &self.wallet,
        );

        let sell_signature = timeout(tx_timeout, sell_future)
            .await
            .context("Таймаут при выполнении продажи")?
            .context("Ошибка выполнения продажи")?;

        Ok((buy_signature, sell_signature))
    }
}

