use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::Json,
};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::str::FromStr;

use crate::web::state::{BotStatus, Metrics, TradeRecord, WebState};

/// Ответ статуса бота
#[derive(Serialize)]
pub struct StatusResponse {
    pub status: String,
    pub simulation_mode: bool,
    pub uptime_seconds: u64,
    pub version: String,
}

/// Ответ баланса кошелька
#[derive(Serialize)]
pub struct BalanceResponse {
    pub sol_balance: String,
    pub usd_equivalent: String,
    pub min_balance_sol: String,
}

/// Ответ арбитражных возможностей
#[derive(Serialize)]
pub struct OpportunitiesResponse {
    pub opportunities: Vec<OpportunityItem>,
    pub count: usize,
    pub timestamp: String,
}

/// Арбитражная возможность
#[derive(Serialize)]
pub struct OpportunityItem {
    pub from_dex: String,
    pub to_dex: String,
    pub base_token: String,
    pub quote_token: String,
    pub buy_price: String,
    pub sell_price: String,
    pub profit_percent: String,
    pub profit_percent_after_fees: String,
    pub trade_amount: String,
    pub estimated_fees: String,
}

/// Параметры запроса для opportunities
#[derive(Deserialize)]
pub struct OpportunitiesQuery {
    pub limit: Option<usize>,
    pub min_profit: Option<f64>,
}

/// Ответ истории сделок
#[derive(Serialize)]
pub struct HistoryResponse {
    pub trades: Vec<TradeItem>,
    pub total: usize,
    pub limit: usize,
    pub offset: usize,
}

/// Параметры запроса для history
#[derive(Deserialize)]
pub struct HistoryQuery {
    pub limit: Option<usize>,
    pub offset: Option<usize>,
    pub from_dex: Option<String>,
    pub status: Option<String>,
}

/// Запись о сделке для API
#[derive(Serialize)]
pub struct TradeItem {
    pub id: String,
    pub timestamp: String,
    pub from_dex: String,
    pub to_dex: String,
    pub base_token: String,
    pub quote_token: String,
    pub amount: String,
    pub profit_percent: String,
    pub profit_sol: String,
    pub status: String,
    pub tx_signature: Option<String>,
}

/// Ответ метрик
#[derive(Serialize)]
pub struct MetricsResponse {
    pub total_trades: u64,
    pub successful_trades: u64,
    pub failed_trades: u64,
    pub total_profit_sol: String,
    pub total_profit_usd: String,
    pub average_profit_percent: String,
    pub rpc_calls_count: u64,
    pub rpc_errors_count: u64,
    pub average_response_time_ms: f64,
    pub last_trade_timestamp: Option<String>,
}

/// Ответ конфигурации (без секретов)
#[derive(Serialize)]
pub struct ConfigResponse {
    pub network: NetworkConfigResponse,
    pub arbitrage: ArbitrageConfigResponse,
    pub dex: DexConfigResponse,
    pub monitoring: MonitoringConfigResponse,
    pub safety: SafetyConfigResponse,
}

#[derive(Serialize)]
pub struct NetworkConfigResponse {
    pub rpc_url: String,
    pub commitment: String,
}

#[derive(Serialize)]
pub struct ArbitrageConfigResponse {
    pub min_profit_percent: f64,
    pub max_trade_amount_sol: f64,
    pub slippage_tolerance: f64,
}

#[derive(Serialize)]
pub struct DexConfigResponse {
    pub enabled_dexes: Vec<String>,
    pub trading_pairs: Vec<String>,
}

#[derive(Serialize)]
pub struct MonitoringConfigResponse {
    pub check_interval_ms: u64,
    pub log_level: String,
}

#[derive(Serialize)]
pub struct SafetyConfigResponse {
    pub simulation_mode: bool,
    pub max_consecutive_failures: u32,
    pub min_balance_sol: f64,
}

/// Ответ управления
#[derive(Serialize)]
pub struct ControlResponse {
    pub status: String,
    pub message: String,
}

/// GET /api/status
pub async fn get_status(State(state): State<WebState>) -> Result<Json<StatusResponse>, StatusCode> {
    let status = *state.bot_status.lock().await;
    let status_str = match status {
        BotStatus::Running => "running",
        BotStatus::Stopped => "stopped",
        BotStatus::Error => "error",
    };

    Ok(Json(StatusResponse {
        status: status_str.to_string(),
        simulation_mode: state.config.safety.simulation_mode,
        uptime_seconds: state.uptime_seconds(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    }))
}

/// GET /api/balance
pub async fn get_balance(
    State(state): State<WebState>,
) -> Result<Json<BalanceResponse>, StatusCode> {
    let balance_lamports = state
        .wallet
        .get_balance(&state.config.network.rpc_url)
        .await
        .map_err(|e| {
            log::error!("Ошибка получения баланса: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    let balance_sol = balance_lamports as f64 / 1_000_000_000.0;
    
    // Примерный курс SOL/USD (можно получать из API)
    let sol_price_usd = 100.0; // TODO: получать реальный курс
    let usd_equivalent = balance_sol * sol_price_usd;

    Ok(Json(BalanceResponse {
        sol_balance: format!("{:.9}", balance_sol),
        usd_equivalent: format!("{:.2}", usd_equivalent),
        min_balance_sol: format!("{:.9}", state.config.safety.min_balance_sol),
    }))
}

/// GET /api/opportunities
pub async fn get_opportunities(
    State(state): State<WebState>,
    Query(params): Query<OpportunitiesQuery>,
) -> Result<Json<OpportunitiesResponse>, StatusCode> {
    let engine_guard = state.arbitrage_engine.lock().await;
    let engine = &*engine_guard;

    let mut opportunities = engine
        .find_opportunities()
        .await
        .map_err(|e| {
            log::error!("Ошибка поиска возможностей: {}", e);
            StatusCode::INTERNAL_SERVER_ERROR
        })?;

    // Фильтрация по минимальной прибыли
    if let Some(min_profit) = params.min_profit {
        let min_profit_decimal = Decimal::from_str(&format!("{:.10}", min_profit))
            .unwrap_or(Decimal::ZERO);
        opportunities.retain(|opp| opp.profit_percent_after_fees >= min_profit_decimal);
    }

    // Ограничение количества
    let limit = params.limit.unwrap_or(10);
    opportunities.truncate(limit);

    let items: Vec<OpportunityItem> = opportunities
        .into_iter()
        .map(|opp| OpportunityItem {
            from_dex: opp.from_dex,
            to_dex: opp.to_dex,
            base_token: opp.base_token,
            quote_token: opp.quote_token,
            buy_price: opp.buy_price.to_string(),
            sell_price: opp.sell_price.to_string(),
            profit_percent: opp.profit_percent.to_string(),
            profit_percent_after_fees: opp.profit_percent_after_fees.to_string(),
            trade_amount: opp.trade_amount.to_string(),
            estimated_fees: opp.estimated_fees.to_string(),
        })
        .collect();

    Ok(Json(OpportunitiesResponse {
        count: items.len(),
        opportunities: items,
        timestamp: chrono::Utc::now().to_rfc3339(),
    }))
}

/// GET /api/history
pub async fn get_history(
    State(state): State<WebState>,
    Query(params): Query<HistoryQuery>,
) -> Result<Json<HistoryResponse>, StatusCode> {
    let history_guard = state.trade_history.lock().await;
    let mut trades: Vec<TradeItem> = history_guard
        .iter()
        .cloned()
        .filter(|trade| {
            // Фильтр по DEX
            if let Some(ref from_dex) = params.from_dex {
                if trade.from_dex != *from_dex {
                    return false;
                }
            }
            // Фильтр по статусу
            if let Some(ref status) = params.status {
                let trade_status = match trade.status {
                    crate::web::state::TradeStatus::Success => "success",
                    crate::web::state::TradeStatus::Failed => "failed",
                    crate::web::state::TradeStatus::Simulated => "simulated",
                };
                if trade_status != status {
                    return false;
                }
            }
            true
        })
        .map(|trade| TradeItem {
            id: trade.id.to_string(),
            timestamp: trade.timestamp.to_rfc3339(),
            from_dex: trade.from_dex,
            to_dex: trade.to_dex,
            base_token: trade.base_token,
            quote_token: trade.quote_token,
            amount: trade.amount.to_string(),
            profit_percent: trade.profit_percent.to_string(),
            profit_sol: trade.profit_sol.to_string(),
            status: match trade.status {
                crate::web::state::TradeStatus::Success => "success".to_string(),
                crate::web::state::TradeStatus::Failed => "failed".to_string(),
                crate::web::state::TradeStatus::Simulated => "simulated".to_string(),
            },
            tx_signature: trade.tx_signature,
        })
        .collect();

    // Сортировка по времени (новые первыми)
    trades.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

    let total = trades.len();
    let offset = params.offset.unwrap_or(0);
    let limit = params.limit.unwrap_or(50);

    // Применение пагинации
    let paginated_trades: Vec<TradeItem> = trades
        .into_iter()
        .skip(offset)
        .take(limit)
        .collect();

    Ok(Json(HistoryResponse {
        total,
        limit,
        offset,
        trades: paginated_trades,
    }))
}

/// GET /api/metrics
pub async fn get_metrics(
    State(state): State<WebState>,
) -> Result<Json<MetricsResponse>, StatusCode> {
    let metrics_guard = state.metrics.lock().await;
    let metrics = metrics_guard.clone();

    Ok(Json(MetricsResponse {
        total_trades: metrics.total_trades,
        successful_trades: metrics.successful_trades,
        failed_trades: metrics.failed_trades,
        total_profit_sol: metrics.total_profit_sol.to_string(),
        total_profit_usd: metrics.total_profit_usd.to_string(),
        average_profit_percent: metrics.average_profit_percent.to_string(),
        rpc_calls_count: metrics.rpc_calls_count,
        rpc_errors_count: metrics.rpc_errors_count,
        average_response_time_ms: metrics.average_response_time_ms,
        last_trade_timestamp: metrics.last_trade_timestamp.map(|dt| dt.to_rfc3339()),
    }))
}

/// GET /api/config
pub async fn get_config(
    State(state): State<WebState>,
) -> Result<Json<ConfigResponse>, StatusCode> {
    Ok(Json(ConfigResponse {
        network: NetworkConfigResponse {
            rpc_url: state.config.network.rpc_url.clone(),
            commitment: state.config.network.commitment.clone(),
        },
        arbitrage: ArbitrageConfigResponse {
            min_profit_percent: state.config.arbitrage.min_profit_percent,
            max_trade_amount_sol: state.config.arbitrage.max_trade_amount_sol,
            slippage_tolerance: state.config.arbitrage.slippage_tolerance,
        },
        dex: DexConfigResponse {
            enabled_dexes: state.config.dex.enabled_dexes.clone(),
            trading_pairs: state.config.dex.trading_pairs.clone(),
        },
        monitoring: MonitoringConfigResponse {
            check_interval_ms: state.config.monitoring.check_interval_ms,
            log_level: state.config.monitoring.log_level.clone(),
        },
        safety: SafetyConfigResponse {
            simulation_mode: state.config.safety.simulation_mode,
            max_consecutive_failures: state.config.safety.max_consecutive_failures,
            min_balance_sol: state.config.safety.min_balance_sol,
        },
    }))
}

/// POST /api/control/start
pub async fn control_start(
    State(state): State<WebState>,
) -> Result<Json<ControlResponse>, StatusCode> {
    let mut status = state.bot_status.lock().await;
    *status = BotStatus::Running;
    
    Ok(Json(ControlResponse {
        status: "started".to_string(),
        message: "Бот запущен".to_string(),
    }))
}

/// POST /api/control/stop
pub async fn control_stop(
    State(state): State<WebState>,
) -> Result<Json<ControlResponse>, StatusCode> {
    let mut status = state.bot_status.lock().await;
    *status = BotStatus::Stopped;
    
    Ok(Json(ControlResponse {
        status: "stopped".to_string(),
        message: "Бот остановлен".to_string(),
    }))
}

/// POST /api/config/reload
pub async fn config_reload(
    State(state): State<WebState>,
) -> Result<Json<ControlResponse>, StatusCode> {
    // Перезагрузка конфигурации (только перезагрузка из файлов, не изменение)
    // В будущем можно реализовать перезагрузку конфига
    log::info!("Запрос на перезагрузку конфигурации (пока не реализовано)");
    
    Ok(Json(ControlResponse {
        status: "reloaded".to_string(),
        message: "Конфигурация перезагружена".to_string(),
    }))
}

/// GET /health
pub async fn health_check() -> Json<HashMap<&'static str, String>> {
    let mut response = HashMap::new();
    response.insert("status", "healthy".to_string());
    response.insert("timestamp", chrono::Utc::now().to_rfc3339());
    Json(response)
}

