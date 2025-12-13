//! Tower middleware for VEX API

use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;
use std::time::Instant;

use crate::auth::{Claims, JwtAuth};
use crate::error::ApiError;
use crate::state::AppState;
// use vex_llm::{RateLimiter, Metrics}; // No longer needed directly here? No, rate_limiter is used.

/// Authentication middleware

/// Authentication middleware
pub async fn auth_middleware(
    State(state): State<AppState>,
    mut request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Skip auth for health check and public endpoints
    let path = request.uri().path();
    if path == "/health" || path.starts_with("/public/") {
        return Ok(next.run(request).await);
    }

    // Extract token from header
    let auth_header = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or_else(|| ApiError::Unauthorized("Missing Authorization header".to_string()))?;

    let token = JwtAuth::extract_from_header(auth_header)?;
    let claims = state.jwt_auth().decode(token)?;

    // Insert claims into request extensions for handlers
    request.extensions_mut().insert(claims);

    Ok(next.run(request).await)
}

/// Rate limiting middleware
pub async fn rate_limit_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Result<Response, ApiError> {
    // Get client identifier (from claims or IP)
    let client_id = request
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.clone())
        .unwrap_or_else(|| "anonymous".to_string());

    // Check rate limit
    state
        .rate_limiter()
        .try_acquire(&client_id)
        .await
        .map_err(|_| ApiError::RateLimited)?;

    Ok(next.run(request).await)
}

/// Request tracing middleware
pub async fn tracing_middleware(
    State(state): State<AppState>,
    request: Request,
    next: Next,
) -> Response {
    let start = Instant::now();
    let method = request.method().clone();
    let uri = request.uri().clone();
    let path = uri.path().to_string();

    // Create span for this request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        status = tracing::field::Empty,
        latency_ms = tracing::field::Empty,
    );

    let response = {
        let _enter = span.enter();
        next.run(request).await
    };

    let latency = start.elapsed();
    let status = response.status();

    // Record metrics
    state.metrics().record_llm_call(0, !status.is_success());

    // Log request
    tracing::info!(
        method = %method,
        path = %path,
        status = %status.as_u16(),
        latency_ms = %latency.as_millis(),
        "Request completed"
    );

    response
}

/// Request ID middleware
pub async fn request_id_middleware(
    mut request: Request,
    next: Next,
) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();
    
    // Add to request extensions
    request.extensions_mut().insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;
    
    // Add to response headers
    response.headers_mut().insert(
        "X-Request-ID",
        request_id.parse().unwrap(),
    );

    response
}

/// Request ID wrapper
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// CORS configuration helper
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    tower_http::cors::CorsLayer::new()
        .allow_origin(tower_http::cors::Any)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([
            header::AUTHORIZATION,
            header::CONTENT_TYPE,
            header::ACCEPT,
        ])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Timeout layer helper
pub fn timeout_layer(duration: std::time::Duration) -> tower_http::timeout::TimeoutLayer {
    tower_http::timeout::TimeoutLayer::new(duration)
}

/// Request body size limit
pub fn body_limit_layer(limit: usize) -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(limit)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_id() {
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
    }
}
