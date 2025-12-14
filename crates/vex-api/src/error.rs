//! API error types with proper HTTP mapping

use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use serde::Serialize;

/// API result type alias
pub type ApiResult<T> = Result<T, ApiError>;

/// Comprehensive API error type
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("Bad request: {0}")]
    BadRequest(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Forbidden: {0}")]
    Forbidden(String),

    #[error("Conflict: {0}")]
    Conflict(String),

    #[error("Rate limited")]
    RateLimited,

    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Circuit open: {0}")]
    CircuitOpen(String),

    #[error("Timeout")]
    Timeout,

    #[error("Validation error: {0}")]
    Validation(String),
}

/// Error response body
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: ErrorBody,
}

#[derive(Debug, Serialize)]
pub struct ErrorBody {
    pub code: String,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, code, message) = match &self {
            ApiError::NotFound(msg) => (StatusCode::NOT_FOUND, "NOT_FOUND", msg.clone()),
            ApiError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "BAD_REQUEST", msg.clone()),
            ApiError::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, "UNAUTHORIZED", msg.clone()),
            ApiError::Forbidden(msg) => (StatusCode::FORBIDDEN, "FORBIDDEN", msg.clone()),
            ApiError::Conflict(msg) => (StatusCode::CONFLICT, "CONFLICT", msg.clone()),
            ApiError::RateLimited => (
                StatusCode::TOO_MANY_REQUESTS,
                "RATE_LIMITED",
                "Too many requests".to_string(),
            ),
            ApiError::ServiceUnavailable(msg) => (
                StatusCode::SERVICE_UNAVAILABLE,
                "SERVICE_UNAVAILABLE",
                msg.clone(),
            ),
            ApiError::Internal(msg) => {
                // Don't expose internal errors to clients
                tracing::error!(error = %msg, "Internal error");
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "INTERNAL_ERROR",
                    "An internal error occurred".to_string(),
                )
            }
            ApiError::CircuitOpen(msg) => {
                (StatusCode::SERVICE_UNAVAILABLE, "CIRCUIT_OPEN", msg.clone())
            }
            ApiError::Timeout => (
                StatusCode::GATEWAY_TIMEOUT,
                "TIMEOUT",
                "Request timed out".to_string(),
            ),
            ApiError::Validation(msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "VALIDATION_ERROR",
                msg.clone(),
            ),
        };

        let body = ErrorResponse {
            error: ErrorBody {
                code: code.to_string(),
                message,
                details: None,
            },
        };

        (status, Json(body)).into_response()
    }
}

// Convenient conversions
impl From<std::io::Error> for ApiError {
    fn from(e: std::io::Error) -> Self {
        ApiError::Internal(e.to_string())
    }
}

impl From<serde_json::Error> for ApiError {
    fn from(e: serde_json::Error) -> Self {
        ApiError::BadRequest(format!("JSON error: {}", e))
    }
}

impl From<vex_llm::LlmError> for ApiError {
    fn from(e: vex_llm::LlmError) -> Self {
        match e {
            vex_llm::LlmError::ConnectionFailed(_) => {
                ApiError::ServiceUnavailable("LLM service unavailable".to_string())
            }
            vex_llm::LlmError::RequestFailed(msg) => ApiError::Internal(msg),
            vex_llm::LlmError::InvalidResponse(msg) => ApiError::Internal(msg),
            vex_llm::LlmError::RateLimited => ApiError::RateLimited,
            vex_llm::LlmError::NotAvailable => {
                ApiError::ServiceUnavailable("LLM provider not available".to_string())
            }
        }
    }
}

impl From<vex_persist::StorageError> for ApiError {
    fn from(e: vex_persist::StorageError) -> Self {
        match e {
            vex_persist::StorageError::NotFound(msg) => ApiError::NotFound(msg),
            vex_persist::StorageError::AlreadyExists(msg) => ApiError::Conflict(msg),
            _ => ApiError::Internal(e.to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[allow(unused_imports)]
    use axum::body::Body;
    use http_body_util::BodyExt;

    #[tokio::test]
    async fn test_error_response() {
        let error = ApiError::NotFound("User not found".to_string());
        let response = error.into_response();

        assert_eq!(response.status(), StatusCode::NOT_FOUND);

        let body = response.into_body();
        let bytes = body.collect().await.unwrap().to_bytes();
        let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();

        assert_eq!(json["error"]["code"], "NOT_FOUND");
    }
}
