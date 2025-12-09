use axum::{
    extract::Request,
    http::StatusCode,
    middleware,
    response::{IntoResponse, Response},
    routing::{get, post},
    Router,
};
use tower::ServiceBuilder;
use tower_http::{
    cors::{Any, CorsLayer},
    trace::TraceLayer,
    services::ServeDir,
};
use std::net::SocketAddr;

use crate::web::state::WebState;
use crate::web::handlers;
use crate::web::websocket;
use crate::web::auth;
use crate::config::Config;

/// Запуск веб-сервера
pub async fn start_server(
    state: WebState,
    config: &Config,
) -> anyhow::Result<()> {
    let bind_address = format!("{}:{}", config.web.bind_address, config.web.port);
    let addr: SocketAddr = bind_address
        .parse()
        .context(format!("Неверный адрес: {}", bind_address))?;

    log::info!("Запуск веб-сервера на http://{}", addr);

    // Создание роутера
    let app = create_router(state);

    // Запуск сервера
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .context(format!("Не удалось привязать адрес: {}", addr))?;

    axum::serve(listener, app)
        .await
        .context("Ошибка веб-сервера")?;

    Ok(())
}

/// Создание роутера с маршрутами
fn create_router(state: WebState) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Публичные маршруты
    let public_routes = Router::new()
        .route("/health", get(handlers::health_check));

    // Защищённые маршруты
    let protected_routes = Router::new()
        .route("/api/status", get(handlers::get_status))
        .route("/api/balance", get(handlers::get_balance))
        .route("/api/opportunities", get(handlers::get_opportunities))
        .route("/api/history", get(handlers::get_history))
        .route("/api/metrics", get(handlers::get_metrics))
        .route("/api/config", get(handlers::get_config))
        .route("/api/control/start", post(handlers::control_start))
        .route("/api/control/stop", post(handlers::control_stop))
        .route("/api/config/reload", post(handlers::config_reload))
        .layer(middleware::from_fn(auth::auth_middleware));

    // WebSocket маршруты (аутентификация внутри handlers)
    let ws_routes = Router::new()
        .route("/ws/updates", get(websocket::ws_updates_handler))
        .route("/ws/logs", get(websocket::ws_logs_handler));

    // Статические файлы (из конфигурации) - fallback для всех остальных запросов
    let static_dir = state.config.web.static_dir.clone();
    let static_service = ServeDir::new(static_dir)
        .append_index_html_on_directories(true);

    // Объединение всех маршрутов
    // Важно: статические файлы должны быть последними (fallback)
    Router::new()
        .merge(public_routes)
        .merge(protected_routes)
        .merge(ws_routes)
        .fallback_service(static_service)
        .layer(
            ServiceBuilder::new()
                .layer(TraceLayer::new_for_http())
                .layer(cors)
        )
        .with_state(state)
}

// Импорт для context
use anyhow::Context;

