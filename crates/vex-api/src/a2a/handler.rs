//! A2A HTTP handlers
//!
//! Axum handlers for A2A protocol endpoints.

use axum::{
    extract::{Path, State},
    http::StatusCode,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use std::collections::HashSet;
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use super::agent_card::AgentCard;
use super::task::{TaskRequest, TaskResponse};

/// Nonce cache for replay protection (2025 best practice)
pub struct NonceCache {
    seen: RwLock<HashSet<String>>,
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
    pub async fn validate(
        &self,
        nonce: Option<&str>,
        timestamp: DateTime<Utc>,
    ) -> Result<(), String> {
        let age = Utc::now().signed_duration_since(timestamp);
        if age > self.max_age {
            return Err(format!("Timestamp too old: {}s", age.num_seconds()));
        }
        if age < Duration::zero() {
            return Err("Timestamp is in the future".to_string());
        }

        if let Some(nonce) = nonce {
            let mut seen = self.seen.write().await;
            if seen.contains(nonce) {
                return Err("Replay detected".to_string());
            }
            seen.insert(nonce.to_string());

            // Cleanup: If cache grows too large, perform a partial cleanup
            // of non-deterministic entries to prevent OOM while maintaining
            // most replay protection. In v0.1.5, we will migrate to a
            // proper TTL-based cache (e.g., moka).
            if seen.len() > 20000 {
                // Drop ~10% randomly if capacity exceeded to avoid total reset
                let total = seen.len();
                let to_remove = total / 10;
                let keys: Vec<String> = seen.iter().take(to_remove).cloned().collect();
                for key in keys {
                    seen.remove(&key);
                }
            }
        }
        Ok(())
    }
}

/// Shared state for A2A handlers
pub struct A2aState {
    pub agent_card: RwLock<AgentCard>,
    pub nonce_mgr: NonceCache,
}

impl Default for A2aState {
    fn default() -> Self {
        Self {
            agent_card: RwLock::new(AgentCard::vex_default()),
            nonce_mgr: NonceCache::default(),
        }
    }
}

/// Health check for A2A protocol (Agent Card)
#[utoipa::path(
    get,
    path = "/.well-known/agent.json",
    responses(
        (status = 200, description = "A2A Agent Card", body = AgentCard)
    )
)]
pub async fn agent_card_handler(State(a2a_state): State<Arc<A2aState>>) -> Json<AgentCard> {
    Json(a2a_state.agent_card.read().await.clone())
}

/// Create a new task (A2A)
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
    State(a2a_state): State<Arc<A2aState>>,
    Json(request): Json<TaskRequest>,
) -> Result<(StatusCode, Json<TaskResponse>), (StatusCode, String)> {
    if let Err(e) = a2a_state
        .nonce_mgr
        .validate(request.nonce.as_deref(), request.timestamp)
        .await
    {
        tracing::warn!(task_id = %request.id, error = %e, "A2A replay protection failed");
        return Err((
            StatusCode::BAD_REQUEST,
            format!("Replay protection failed: {}", e),
        ));
    }

    let response = TaskResponse::pending(request.id);
    Ok((StatusCode::ACCEPTED, Json(response)))
}

/// Get task status (A2A)
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
    State(_a2a_state): State<Arc<A2aState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, StatusCode> {
    let response = TaskResponse::pending(task_id);
    Ok(Json(response))
}

/// Build A2A routes decoupled from main AppState
pub fn a2a_routes() -> Router<Arc<A2aState>> {
    use axum::routing::{get, post};

    Router::new()
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
        let app = a2a_routes().with_state(state);

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
        let app = a2a_routes().with_state(state);

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
        let app = a2a_routes().with_state(state);
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
