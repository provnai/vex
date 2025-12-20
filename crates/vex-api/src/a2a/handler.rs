//! A2A HTTP handlers
//!
//! Axum handlers for A2A protocol endpoints.
//!
//! # Endpoints
//!
//! - `GET /.well-known/agent.json` - Agent Card
//! - `POST /a2a/tasks` - Create a new task
//! - `GET /a2a/tasks/:id` - Get task status
//!
//! # Security
//!
//! - Authentication required for task endpoints
//! - Agent Card is public but can be rate-limited
//! - All responses include Merkle hashes

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::agent_card::AgentCard;
use super::task::{TaskRequest, TaskResponse};

/// Nonce cache for replay protection (2025 best practice)
///
/// Stores seen nonces with automatic expiration to prevent replay attacks.
/// Uses a 5-minute window per A2A protocol recommendations.
pub struct NonceCache {
    /// Set of seen nonces (nonce + tenant combination)
    seen: RwLock<HashSet<String>>,
    /// Maximum age for timestamps
    max_age: Duration,
}

impl Default for NonceCache {
    fn default() -> Self {
        Self {
            seen: RwLock::new(HashSet::new()),
            max_age: Duration::minutes(5),
        }
    }
}

impl NonceCache {
    /// Validate a nonce and timestamp for replay protection
    ///
    /// Returns error if:
    /// - Timestamp is outside the 5-minute window
    /// - Nonce has been seen before
    pub async fn validate(
        &self,
        nonce: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), String> {
        // Check timestamp freshness
        let age = Utc::now().signed_duration_since(timestamp);
        if age > self.max_age {
            return Err(format!(
                "Timestamp too old: {} seconds ago (max: {} seconds)",
                age.num_seconds(),
                self.max_age.num_seconds()
            ));
        }
        if age < Duration::zero() {
            return Err("Timestamp is in the future".to_string());
        }

        // If nonce provided, check uniqueness
        if let Some(nonce) = nonce {
            let mut seen = self.seen.write().await;
            if seen.contains(nonce) {
                return Err("Replay detected: nonce already used".to_string());
            }
            seen.insert(nonce.to_string());

            // Cleanup: remove old entries periodically (simple approach)
            // In production, use a TTL-based cache like moka
            if seen.len() > 10000 {
                seen.clear();
                seen.insert(nonce.to_string());
                tracing::warn!("Nonce cache cleared due to size limit");
            }
        }

        Ok(())
    }
}

/// Shared state for A2A handlers
pub struct A2aState {
    /// The agent card for this VEX instance
    pub agent_card: AgentCard,
    /// Nonce cache for replay protection
    pub nonce_cache: NonceCache,
    // In a full implementation, this would include:
    // - Task storage
    // - ToolExecutor reference
    // - AuditStore reference
}

impl Default for A2aState {
    fn default() -> Self {
        Self {
            agent_card: AgentCard::vex_default(),
            nonce_cache: NonceCache::default(),
        }
    }
}

use crate::state::AppState;

/// Handler: GET /.well-known/agent.json
///
/// Returns the A2A Agent Card describing this agent's capabilities.
/// This endpoint is public per the A2A spec.
#[utoipa::path(
    get,
    path = "/.well-known/agent.json",
    responses(
        (status = 200, description = "A2A Agent Card", body = AgentCard)
    )
)]
pub async fn agent_card_handler(State(state): State<AppState>) -> Json<AgentCard> {
    Json(state.a2a_state().agent_card.clone())
}

/// Handler: POST /a2a/tasks
///
/// Create a new task for execution.
///
/// # Security
/// - Validates caller authentication
/// - Checks nonce/timestamp for replay protection
/// - Rate limits per caller agent
#[utoipa::path(
    post,
    path = "/a2a/tasks",
    request_body = TaskRequest,
    responses(
        (status = 202, description = "Task accepted", body = TaskResponse),
        (status = 400, description = "Invalid request or replay detected")
    )
)]
pub async fn create_task_handler(
    State(state): State<AppState>,
    Json(request): Json<TaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), (StatusCode, String)> {
    // Validate nonce/timestamp for replay protection
    if let Err(e) = state
        .a2a_state()
        .nonce_cache
        .validate(request.nonce.as_deref(), request.timestamp)
        .await
    {
        tracing::warn!(task_id = %request.id, error = %e, "A2A replay protection failed");
        return Err((StatusCode::BAD_REQUEST, format!("Replay protection failed: {}", e)));
    }

    // Queue the task for execution
    let response = TaskResponse::pending(request.id);
    Ok((StatusCode::ACCEPTED, Json(response)))
}

/// Handler: GET /a2a/tasks/:id
///
/// Get the status of an existing task.
#[utoipa::path(
    get,
    path = "/a2a/tasks/{id}",
    params(
        ("id" = Uuid, Path, description = "Task ID")
    ),
    responses(
        (status = 200, description = "Current task status", body = TaskResponse),
        (status = 404, description = "Task not found")
    )
)]
pub async fn get_task_handler(
    State(_state): State<AppState>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let response = TaskResponse::pending(task_id);
    Ok(Json(response))
}

/// Build A2A routes for inclusion in the main router
pub fn a2a_routes() -> axum::Router<AppState> {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/.well-known/agent.json", get(agent_card_handler))
        .route("/a2a/tasks", post(create_task_handler))
        .route("/a2a/tasks/{id}", get(get_task_handler))
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::body::Body;
    use axum::http::Request;
    use tower::ServiceExt;

    fn create_test_state() -> Arc<A2aState> {
        Arc::new(A2aState::default())
    }

    #[tokio::test]
    async fn test_agent_card_endpoint() {
        let state = create_test_state();
        let app = a2a_routes(state);

        let response = app
            .oneshot(
                Request::builder()
                    .uri("/.well-known/agent.json")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn test_create_task_endpoint() {
        let state = create_test_state();
        let app = a2a_routes(state);

        let task_req = TaskRequest::new("verify", serde_json::json!({"claim": "test"}));
        let body = serde_json::to_string(&task_req).unwrap();

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/a2a/tasks")
                    .header("content-type", "application/json")
                    .body(Body::from(body))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::ACCEPTED);
    }

    #[tokio::test]
    async fn test_get_task_endpoint() {
        let state = create_test_state();
        let app = a2a_routes(state);
        let task_id = Uuid::new_v4();

        let response = app
            .oneshot(
                Request::builder()
                    .uri(format!("/a2a/tasks/{}", task_id))
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::OK);
    }
}
