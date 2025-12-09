pub mod state;
pub mod handlers;
pub mod websocket;
pub mod auth;
pub mod server;

use crate::config::Config;
use crate::monitor::Monitor;
use crate::arbitrage::ArbitrageEngine;
use crate::wallet::Wallet;
use std::sync::Arc;

/// Создание состояния веб-сервера
pub fn create_state(
    config: Config,
    monitor: Monitor,
    wallet: Arc<Wallet>,
    arbitrage_engine: Arc<tokio::sync::Mutex<ArbitrageEngine>>,
) -> state::WebState {
    state::WebState::new(config, monitor, wallet, arbitrage_engine)
}

/// Запуск веб-сервера
pub async fn start_server(
    state: state::WebState,
    config: &Config,
) -> anyhow::Result<()> {
    server::start_server(state, config).await
}

