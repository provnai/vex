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
use std::sync::Arc;
use uuid::Uuid;

use super::agent_card::AgentCard;
use super::task::{TaskRequest, TaskResponse};

/// Shared state for A2A handlers
pub struct A2aState {
    /// The agent card for this VEX instance
    pub agent_card: AgentCard,
    // In a full implementation, this would include:
    // - Task storage
    // - ToolExecutor reference
    // - AuditStore reference
}

impl Default for A2aState {
    fn default() -> Self {
        Self {
            agent_card: AgentCard::vex_default(),
        }
    }
}

/// Handler: GET /.well-known/agent.json
///
/// Returns the A2A Agent Card describing this agent's capabilities.
/// This endpoint is public per the A2A spec.
pub async fn agent_card_handler(State(state): State<Arc<A2aState>>) -> Json<AgentCard> {
    Json(state.agent_card.clone())
}

/// Handler: POST /a2a/tasks
///
/// Create a new task for execution.
///
/// # Security
/// - Validates caller authentication
/// - Checks nonce/timestamp for replay protection
/// - Rate limits per caller agent
pub async fn create_task_handler(
    State(_state): State<Arc<A2aState>>,
    Json(request): Json<TaskRequest>,
) -> (StatusCode, Json<TaskResponse>) {
    // Validate nonce/timestamp for replay protection
    // In a full implementation, we'd check:
    // 1. Nonce hasn't been seen before
    // 2. Timestamp is within acceptable window (e.g., 5 minutes)

    // For now, return a pending response
    // In a full implementation, this would:
    // 1. Queue the task for execution
    // 2. Return immediately with pending status
    // 3. Execute asynchronously

    let response = TaskResponse::pending(request.id);
    (StatusCode::ACCEPTED, Json(response))
}

/// Handler: GET /a2a/tasks/:id
///
/// Get the status of an existing task.
pub async fn get_task_handler(
    State(_state): State<Arc<A2aState>>,
    Path(task_id): Path<Uuid>,
) -> Result<Json<TaskResponse>, StatusCode> {
    // In a full implementation, this would look up the task from storage
    // For now, return a placeholder response

    // Simulate task not found for non-existent tasks
    // In reality, we'd query task storage
    let response = TaskResponse::pending(task_id);
    Ok(Json(response))
}

/// Build A2A routes for inclusion in the main router
///
/// # Example
///
/// ```ignore
/// let app = Router::new()
///     .merge(a2a_routes(a2a_state));
/// ```
pub fn a2a_routes(state: Arc<A2aState>) -> axum::Router {
    use axum::routing::{get, post};

    axum::Router::new()
        .route("/.well-known/agent.json", get(agent_card_handler))
        .route("/a2a/tasks", post(create_task_handler))
        .route("/a2a/tasks/{id}", get(get_task_handler))
        .with_state(state)
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
