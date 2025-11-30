// monitor.rs — мониторинг состояния бота
// Назначение: логирование событий, ошибок, метрик, алерты (в будущем)

use crate::config::Config;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;
use log::{info, warn, error};

pub struct Monitor {
    max_consecutive_failures: u32,
    current_failures: AtomicU32,
}

impl Monitor {
    pub fn new(config: &Config) -> Self {
        Self {
            max_consecutive_failures: config.safety.max_consecutive_failures,
            current_failures: AtomicU32::new(0),
        }
    }
    
    /// Логирование успешной арбитражной сделки
    pub fn log_success(&self, profit_percent: f64) {
        self.current_failures.store(0, Ordering::SeqCst);
        info!("✅ Успешная арбитражная сделка, профит: {:.4}%", profit_percent);
    }
    
    /// Логирование ошибки и проверка, не пора ли останавливать бота
    pub fn log_failure(&self, context: &str, error_msg: &str) -> bool {
        let failures = self.current_failures.fetch_add(1, Ordering::SeqCst) + 1;
        
        warn!(
            "❌ Ошибка в контексте '{}': {} (подряд ошибок: {})",
            context, error_msg, failures
        );
        
        if failures >= self.max_consecutive_failures {
            error!(
                "🚨 Достигнут лимит подряд идущих ошибок ({}). Рекомендуется остановить бота.",
                self.max_consecutive_failures
            );
            true
        } else {
            false
        }
    }
    
    /// Логирование произвольного инфо-сообщения
    pub fn log_info(&self, msg: &str) {
        info!("{}", msg);
    }
    
    /// Логирование произвольного предупреждения
    pub fn log_warn(&self, msg: &str) {
        warn!("{}", msg);
    }
}