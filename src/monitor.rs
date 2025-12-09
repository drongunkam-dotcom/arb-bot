use log;
use crate::config::Config;
use rust_decimal::Decimal;

/// Система мониторинга и логирования
#[derive(Clone)]
pub struct Monitor {
    config: Config,
}

impl Monitor {
    /// Создание нового монитора
    pub fn new(config: &Config) -> Self {
        Self {
            config: config.clone(),
        }
    }

    /// Логирование арбитражной сделки
    pub fn log_arbitrage(
        &self,
        from_dex: &str,
        to_dex: &str,
        profit_percent: Decimal,
        simulation_mode: bool,
    ) {
        if simulation_mode {
            log::info!(
                "[ARBITRAGE] {} -> {} | Прибыль: {:.2}% | Режим: СИМУЛЯЦИЯ",
                from_dex,
                to_dex,
                profit_percent
            );
        } else {
            log::info!(
                "[ARBITRAGE] {} -> {} | Прибыль: {:.2}% | Режим: ПРОДАКШН",
                from_dex,
                to_dex,
                profit_percent
            );
        }
    }

    /// Логирование ошибки
    pub fn log_error(&self, error: &str) {
        log::error!("[ERROR] {}", error);
    }

    /// Логирование предупреждения
    pub fn log_warning(&self, warning: &str) {
        log::warn!("[WARNING] {}", warning);
    }
}

