//! Shared library for Pact Mock Servers
//!
//! Provides logging and auth helper middlewares for mock servers.

pub mod hpe_oneview;

use axum::{
    extract::Request,
    http::{HeaderValue, StatusCode},
    middleware::Next,
    response::{IntoResponse, Json, Response},
};
use serde_json::{json, Value};
use tracing::{info, warn};

/// Health check endpoint
pub async fn health_check() -> Json<Value> {
    Json(json!({
        "status": "ok",
        "service": "oneview-mock-server"
    }))
}

/// Request logging middleware
pub async fn logging_middleware(request: Request, next: Next) -> Response {
    let method = request.method().clone();
    let path = request.uri().path().to_string();
    let start = std::time::Instant::now();

    info!("→ {} {}", method, path);

    let response = next.run(request).await;
    let duration = start.elapsed();
    let status = response.status();

    info!(
        "← {} {} [{}] [{:.3}s]",
        method,
        path,
        status.as_u16(),
        duration.as_secs_f64()
    );

    response
}

/// Optional auth header verification middleware for mock routes
pub async fn auth_middleware(request: Request, next: Next) -> Response {
    // Skip auth check for health / base paths
    let path = request.uri().path();
    if path == "/health" || path == "/" || path == "/rest/login-sessions" {
        return next.run(request).await;
    }

    if let Some(auth_header) = request.headers().get("auth") {
        if let Ok(token) = auth_header.to_str() {
            if !token.is_empty() {
                return next.run(request).await;
            }
        }
    }

    warn!(
        "Unauthorized: missing or empty 'auth' header on path: {}",
        path
    );
    let mut response = (
        StatusCode::UNAUTHORIZED,
        Json(json!({
            "errorCode": "UNAUTHORIZED",
            "message": "The session token is invalid or has expired."
        })),
    )
        .into_response();

    response
        .headers_mut()
        .insert("content-type", HeaderValue::from_static("application/json"));

    response
}
