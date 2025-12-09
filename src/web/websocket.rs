use axum::{
    extract::{ws::WebSocket, State, Query},
    response::Response,
};
use futures_util::{SinkExt, StreamExt};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;
use tokio::time::{interval, Duration};

use crate::web::state::WebState;

/// Тип WebSocket сообщения
#[derive(Debug, Serialize, Clone)]
#[serde(tag = "type")]
pub enum WsMessage {
    Status {
        status: String,
        simulation_mode: bool,
        uptime_seconds: u64,
    },
    Opportunity {
        from_dex: String,
        to_dex: String,
        base_token: String,
        quote_token: String,
        profit_percent: String,
        profit_percent_after_fees: String,
    },
    Trade {
        id: String,
        timestamp: String,
        from_dex: String,
        to_dex: String,
        profit_percent: String,
        status: String,
    },
    Metrics {
        total_trades: u64,
        successful_trades: u64,
        failed_trades: u64,
        total_profit_sol: String,
    },
    Error {
        message: String,
    },
}

/// Параметры запроса для WebSocket
#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

/// Обработчик WebSocket для обновлений
pub async fn ws_updates_handler(
    ws: WebSocket,
    State(state): State<WebState>,
    Query(params): Query<WsQuery>,
) -> Response {
    // Проверка аутентификации
    if let Some(token) = params.token {
        if !crate::web::auth::verify_ws_token(&token) {
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::UNAUTHORIZED)
                .body("Unauthorized".into())
                .unwrap()
                .into_response();
        }
    } else {
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body("Token required".into())
            .unwrap()
            .into_response();
    }

    ws.on_upgrade(|socket| handle_updates_socket(socket, state))
}

/// Обработка WebSocket соединения для обновлений
async fn handle_updates_socket(socket: WebSocket, state: WebState) {
    let (mut sender, mut receiver) = socket.split();
    let mut interval_timer = interval(Duration::from_secs(5));

    loop {
        tokio::select! {
            // Отправка периодических обновлений
            _ = interval_timer.tick() => {
                // Отправка статуса
                let status = *state.bot_status.lock().await;
                let status_str = match status {
                    crate::web::state::BotStatus::Running => "running",
                    crate::web::state::Stopped => "stopped",
                    crate::web::state::Error => "error",
                };

                let msg = WsMessage::Status {
                    status: status_str.to_string(),
                    simulation_mode: state.config.safety.simulation_mode,
                    uptime_seconds: state.uptime_seconds(),
                };

                let json = serde_json::to_string(&msg).unwrap_or_default();
                if sender.send(axum::extract::ws::Message::Text(json)).await.is_err() {
                    break;
                }

                // Отправка метрик
                let metrics = state.metrics.lock().await.clone();
                let metrics_msg = WsMessage::Metrics {
                    total_trades: metrics.total_trades,
                    successful_trades: metrics.successful_trades,
                    failed_trades: metrics.failed_trades,
                    total_profit_sol: metrics.total_profit_sol.to_string(),
                };

                let metrics_json = serde_json::to_string(&metrics_msg).unwrap_or_default();
                if sender.send(axum::extract::ws::Message::Text(metrics_json)).await.is_err() {
                    break;
                }
            }
            // Получение сообщений от клиента (если нужно)
            msg = receiver.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Обработчик WebSocket для логов
pub async fn ws_logs_handler(
    ws: WebSocket,
    State(_state): State<WebState>,
    Query(params): Query<WsQuery>,
) -> Response {
    // Проверка аутентификации
    if let Some(token) = params.token {
        if !crate::web::auth::verify_ws_token(&token) {
            return axum::response::Response::builder()
                .status(axum::http::StatusCode::UNAUTHORIZED)
                .body("Unauthorized".into())
                .unwrap()
                .into_response();
        }
    } else {
        return axum::response::Response::builder()
            .status(axum::http::StatusCode::UNAUTHORIZED)
            .body("Token required".into())
            .unwrap()
            .into_response();
    }

    // TODO: Реализовать стрим логов через broadcast channel
    ws.on_upgrade(|socket| handle_logs_socket(socket))
}

/// Обработка WebSocket соединения для логов
async fn handle_logs_socket(socket: WebSocket) {
    let (mut sender, mut receiver) = socket.split();

    // Пока отправляем заглушку
    loop {
        tokio::select! {
            msg = receiver.next() => {
                match msg {
                    Some(Ok(axum::extract::ws::Message::Close(_))) => {
                        break;
                    }
                    Some(Err(_)) => {
                        break;
                    }
                    _ => {}
                }
            }
        }
    }
}

