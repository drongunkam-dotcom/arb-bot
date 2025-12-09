use axum::{
    extract::Request,
    http::{header::AUTHORIZATION, StatusCode},
    middleware::Next,
    response::Response,
};
use base64::{engine::general_purpose, Engine as _};
use std::env;

/// Проверка Basic Authentication
pub async fn auth_middleware(mut request: Request, next: Next) -> Result<Response, StatusCode> {
    // Получение заголовка Authorization
    let auth_header = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|h| h.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    // Проверка формата "Basic <credentials>"
    if !auth_header.starts_with("Basic ") {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let credentials = auth_header.trim_start_matches("Basic ");
    
    // Декодирование base64
    let decoded = general_purpose::STANDARD
        .decode(credentials)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;
    
    let credentials_str = String::from_utf8(decoded)
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    // Разделение username:password
    let parts: Vec<&str> = credentials_str.splitn(2, ':').collect();
    if parts.len() != 2 {
        return Err(StatusCode::UNAUTHORIZED);
    }

    let username = parts[0];
    let password = parts[1];

    // Получение учётных данных из переменных окружения
    let expected_username = env::var("WEB_USERNAME")
        .unwrap_or_else(|_| "admin".to_string());
    let expected_password = env::var("WEB_PASSWORD")
        .ok()
        .ok_or(StatusCode::INTERNAL_SERVER_ERROR)?;

    // Проверка учётных данных
    if username == expected_username && password == expected_password {
        Ok(next.run(request).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Проверка аутентификации для WebSocket (через query параметр token)
pub fn verify_ws_token(token: &str) -> bool {
    // Простая проверка токена (в будущем можно заменить на JWT)
    // Для Basic Auth используем base64(username:password) как токен
    let expected_username = env::var("WEB_USERNAME")
        .unwrap_or_else(|_| "admin".to_string());
    let expected_password = env::var("WEB_PASSWORD")
        .ok()
        .unwrap_or_default();

    let expected_token = general_purpose::STANDARD.encode(
        format!("{}:{}", expected_username, expected_password)
    );

    token == expected_token
}

