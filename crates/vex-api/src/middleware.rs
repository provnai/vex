//! Tower middleware for VEX API

use axum::{
    extract::{Request, State},
    http::header,
    middleware::Next,
    response::Response,
};
use std::time::Instant;

use crate::auth::{Claims, JwtAuth};
use crate::error::ApiError;
use crate::state::AppState;
// use vex_llm::{RateLimiter, Metrics}; // No longer needed directly here? No, rate_limiter is used.

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
    // Extract tenant identifier (prioritize authenticated sub from JWT)
    let tenant_id = request
        .extensions()
        .get::<Claims>()
        .map(|c| c.sub.clone())
        .or_else(|| {
            request
                .headers()
                .get("x-client-id")
                .and_then(|h| h.to_str().ok())
                .map(|s| s.to_string())
        })
        .unwrap_or_else(|| "anonymous".to_string());

    // Check rate limit
    state
        .rate_limiter()
        .check(&tenant_id)
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
    // Extract IDs for tracing
    let request_id = request.extensions().get::<RequestId>().map(|id| id.0.clone()).unwrap_or_else(|| "unknown".to_string());
    let tenant_id = request.extensions().get::<Claims>().map(|c| c.sub.clone()).unwrap_or_else(|| "anonymous".to_string());

    // Create span for this request
    let span = tracing::info_span!(
        "http_request",
        method = %method,
        path = %path,
        request_id = %request_id,
        tenant_id = %tenant_id,
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
pub async fn request_id_middleware(mut request: Request, next: Next) -> Response {
    let request_id = uuid::Uuid::new_v4().to_string();

    // Add to request extensions
    request
        .extensions_mut()
        .insert(RequestId(request_id.clone()));

    let mut response = next.run(request).await;

    // Add to response headers
    response
        .headers_mut()
        .insert("X-Request-ID", request_id.parse().unwrap());

    response
}

/// Request ID wrapper
#[derive(Clone, Debug)]
pub struct RequestId(pub String);

/// CORS configuration helper
/// Reads allowed origins from VEX_CORS_ORIGINS env var (comma-separated)
/// Falls back to restrictive default if not set
pub fn cors_layer() -> tower_http::cors::CorsLayer {
    use tower_http::cors::{AllowOrigin, CorsLayer};

    let origins = std::env::var("VEX_CORS_ORIGINS").ok();

    let allow_origin = match origins {
        Some(origins_str) if !origins_str.is_empty() => {
            let origins: Vec<axum::http::HeaderValue> = origins_str
                .split(',')
                .filter_map(|s| s.trim().parse().ok())
                .collect();
            if origins.is_empty() {
                tracing::warn!("VEX_CORS_ORIGINS is set but contains no valid origins, using restrictive default");
                AllowOrigin::exact("https://localhost".parse().unwrap())
            } else {
                tracing::info!("CORS configured for {} origin(s)", origins.len());
                AllowOrigin::list(origins)
            }
        }
        _ => {
            // No CORS_ORIGINS set - use restrictive default for security
            tracing::warn!("VEX_CORS_ORIGINS not set, using restrictive CORS (localhost only)");
            AllowOrigin::exact("https://localhost".parse().unwrap())
        }
    };

    CorsLayer::new()
        .allow_origin(allow_origin)
        .allow_methods([
            axum::http::Method::GET,
            axum::http::Method::POST,
            axum::http::Method::PUT,
            axum::http::Method::DELETE,
            axum::http::Method::OPTIONS,
        ])
        .allow_headers([header::AUTHORIZATION, header::CONTENT_TYPE, header::ACCEPT])
        .max_age(std::time::Duration::from_secs(3600))
}

/// Timeout layer helper
#[allow(deprecated)]
pub fn timeout_layer(duration: std::time::Duration) -> tower_http::timeout::TimeoutLayer {
    tower_http::timeout::TimeoutLayer::new(duration)
}

/// Request body size limit
pub fn body_limit_layer(limit: usize) -> tower_http::limit::RequestBodyLimitLayer {
    tower_http::limit::RequestBodyLimitLayer::new(limit)
}

/// Security headers middleware
/// Adds standard security headers to all responses
pub async fn security_headers_middleware(request: Request, next: Next) -> Response {
    let mut response = next.run(request).await;

    let headers = response.headers_mut();

    // Prevent MIME type sniffing
    headers.insert("X-Content-Type-Options", "nosniff".parse().unwrap());

    // Prevent clickjacking
    headers.insert("X-Frame-Options", "DENY".parse().unwrap());

    // XSS protection (legacy, but still useful)
    headers.insert("X-XSS-Protection", "1; mode=block".parse().unwrap());

    // Content Security Policy
    headers.insert(
        "Content-Security-Policy",
        "default-src 'self'; frame-ancestors 'none'"
            .parse()
            .unwrap(),
    );

    // HSTS - Enable in production by setting VEX_ENABLE_HSTS=1
    if std::env::var("VEX_ENABLE_HSTS").is_ok() {
        headers.insert(
            "Strict-Transport-Security",
            "max-age=31536000; includeSubDomains".parse().unwrap(),
        );
    }

    // Referrer policy
    headers.insert(
        "Referrer-Policy",
        "strict-origin-when-cross-origin".parse().unwrap(),
    );

    // Permissions policy
    headers.insert(
        "Permissions-Policy",
        "geolocation=(), microphone=(), camera=()".parse().unwrap(),
    );

    response
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_request_id() {
        let id1 = uuid::Uuid::new_v4().to_string();
        let id2 = uuid::Uuid::new_v4().to_string();
        assert_ne!(id1, id2);
    }
}
