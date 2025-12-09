use std::sync::Arc;
use tokio::sync::Mutex;
use uuid::Uuid;
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Serialize, Deserialize};

use crate::config::Config;
use crate::monitor::Monitor;
use crate::arbitrage::{ArbitrageEngine, ArbitrageOpportunity};
use crate::wallet::Wallet;

/// Состояние веб-сервера для доступа к данным бота
pub struct WebState {
    pub config: Arc<Config>,
    pub monitor: Arc<Monitor>,
    pub arbitrage_engine: Arc<tokio::sync::Mutex<ArbitrageEngine>>,
    pub wallet: Arc<Wallet>,
    pub metrics: Arc<Mutex<Metrics>>,
    pub trade_history: Arc<Mutex<Vec<TradeRecord>>>,
    pub start_time: DateTime<Utc>,
    pub bot_status: Arc<Mutex<BotStatus>>,
}

/// Метрики производительности
#[derive(Debug, Clone, Default, Serialize)]
pub struct Metrics {
    pub total_trades: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub total_profit_sol: Decimal,
    pub total_profit_usd: Decimal,
    pub average_profit_percent: Decimal,
    pub rpc_calls_count: u64,
    pub rpc_errors_count: u64,
    pub average_response_time_ms: f64,
    pub last_trade_timestamp: Option<DateTime<Utc>>,
}

/// Статус бота
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum BotStatus {
    Running,
    Stopped,
    Error,
}

/// Запись о сделке
#[derive(Debug, Clone, Serialize)]
pub struct TradeRecord {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub from_dex: String,
    pub to_dex: String,
    pub base_token: String,
    pub quote_token: String,
    pub amount: Decimal,
    pub profit_percent: Decimal,
    pub profit_sol: Decimal,
    pub status: TradeStatus,
    pub tx_signature: Option<String>,
}

/// Статус сделки
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum TradeStatus {
    Success,
    Failed,
    Simulated,
}

impl WebState {
    /// Создание нового состояния веб-сервера
    pub fn new(
        config: Config,
        monitor: Monitor,
        wallet: Arc<Wallet>,
        arbitrage_engine: Arc<tokio::sync::Mutex<ArbitrageEngine>>,
    ) -> Self {
        Self {
            config: Arc::new(config),
            monitor: Arc::new(monitor),
            arbitrage_engine,
            wallet,
            metrics: Arc::new(Mutex::new(Metrics::default())),
            trade_history: Arc::new(Mutex::new(Vec::new())),
            start_time: Utc::now(),
            bot_status: Arc::new(Mutex::new(BotStatus::Running)),
        }
    }

    /// Получение uptime в секундах
    pub fn uptime_seconds(&self) -> u64 {
        let now = Utc::now();
        let duration = now.signed_duration_since(self.start_time);
        duration.num_seconds() as u64
    }
}

