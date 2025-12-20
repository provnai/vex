//! API routes for VEX endpoints

use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{ApiError, ApiResult};
use crate::sanitize::{sanitize_name, sanitize_prompt, sanitize_role};
use crate::state::AppState;
use utoipa::OpenApi;
use vex_persist::AgentStore;

/// Health check response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<ComponentHealth>,
}

/// Component health status
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ComponentHealth {
    pub database: ComponentStatus,
    pub queue: ComponentStatus,
}

/// Individual component status
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ComponentStatus {
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latency_ms: Option<u64>,
}

/// Basic health check handler (lightweight)
#[utoipa::path(
    get,
    path = "/health",
    responses(
        (status = 200, description = "Basic health check", body = HealthResponse)
    )
)]
pub async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        components: None,
    })
}

/// Detailed health check with database connectivity
#[utoipa::path(
    get,
    path = "/health/detailed",
    responses(
        (status = 200, description = "Detailed health check with component status", body = HealthResponse)
    )
)]
pub async fn health_detailed(State(state): State<AppState>) -> Json<HealthResponse> {
    let start = std::time::Instant::now();

    // Check database
    let db_healthy = state.db().is_healthy().await;
    let db_latency = start.elapsed().as_millis() as u64;

    // Queue is always healthy (in-memory)
    let queue_status = ComponentStatus {
        status: "healthy".to_string(),
        latency_ms: Some(0),
    };

    let db_status = ComponentStatus {
        status: if db_healthy { "healthy" } else { "unhealthy" }.to_string(),
        latency_ms: Some(db_latency),
    };

    let overall_status = if db_healthy { "healthy" } else { "degraded" };

    Json(HealthResponse {
        status: overall_status.to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        components: Some(ComponentHealth {
            database: db_status,
            queue: queue_status,
        }),
    })
}

/// Agent creation request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct CreateAgentRequest {
    pub name: String,
    pub role: String,
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    #[serde(default)]
    pub spawn_shadow: bool,
}

fn default_max_depth() -> u8 {
    3
}

/// Agent response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct AgentResponse {
    pub id: Uuid,
    pub name: String,
    pub role: String,
    pub generation: u32,
    pub fitness: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

/// Create agent handler
#[utoipa::path(
    post,
    path = "/api/v1/agents",
    request_body = CreateAgentRequest,
    responses(
        (status = 200, description = "Agent created successfully", body = AgentResponse),
        (status = 403, description = "Insufficient permissions"),
        (status = 400, description = "Invalid input")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn create_agent(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<CreateAgentRequest>,
) -> ApiResult<Json<AgentResponse>> {
    // Validate role access
    if !claims.has_role("user") {
        return Err(ApiError::Forbidden("Insufficient permissions".to_string()));
    }

    // Sanitize inputs
    let name = sanitize_name(&req.name)
        .map_err(|e| ApiError::Validation(format!("Invalid name: {}", e)))?;
    let role = sanitize_role(&req.role)
        .map_err(|e| ApiError::Validation(format!("Invalid role: {}", e)))?;

    // Create agent with sanitized inputs
    let config = vex_core::AgentConfig {
        name: name.clone(),
        role: role.clone(),
        max_depth: req.max_depth,
        spawn_shadow: req.spawn_shadow,
    };
    let agent = vex_core::Agent::new(config);

    // Persist agent with tenant isolation
    let store = AgentStore::new(state.db());

    store
        .save(&claims.sub, &agent)
        .await
        .map_err(|e| ApiError::Internal(format!("Failed to save agent: {}", e)))?;

    // Record metrics
    state.metrics().record_agent_created();

    Ok(Json(AgentResponse {
        id: agent.id,
        name: req.name,
        role: req.role,
        generation: agent.generation,
        fitness: agent.fitness,
        created_at: chrono::Utc::now(),
    }))
}

/// Execute agent request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct ExecuteRequest {
    pub prompt: String,
    #[serde(default)]
    pub enable_adversarial: bool,
    #[serde(default = "default_max_rounds")]
    pub max_debate_rounds: u32,
}

fn default_max_rounds() -> u32 {
    3
}

/// Execute agent response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct ExecuteResponse {
    pub agent_id: Uuid,
    pub response: String,
    pub verified: bool,
    pub confidence: f64,
    pub context_hash: String,
    pub latency_ms: u64,
}

/// Execute agent handler
#[utoipa::path(
    post,
    path = "/api/v1/agents/{id}/execute",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    request_body = ExecuteRequest,
    responses(
        (status = 200, description = "Job queued successfully", body = ExecuteResponse),
        (status = 404, description = "Agent not found")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn execute_agent(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
    Json(req): Json<ExecuteRequest>,
) -> ApiResult<Json<ExecuteResponse>> {
    let start = std::time::Instant::now();

    // Sanitize and validate prompt
    let prompt = sanitize_prompt(&req.prompt)
        .map_err(|e| ApiError::Validation(format!("Invalid prompt: {}", e)))?;

    // Check ownership/existence
    let store = AgentStore::new(state.db());

    let exists = store
        .exists(&claims.sub, agent_id)
        .await
        .map_err(|e| ApiError::Internal(format!("Storage error: {}", e)))?;

    if !exists {
        return Err(ApiError::NotFound("Agent not found".to_string()));
    }

    // Create job payload with sanitized prompt
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "prompt": prompt,
        "config": {
            "enable_adversarial": req.enable_adversarial,
            "max_rounds": req.max_debate_rounds
        },
        "tenant_id": claims.sub
    });

    // Enqueue job with explicit type checks
    // Enqueue job via dynamic backend
    let pool = state.queue();

    // For dynamic dispatch, we access the backend. It's Arc<dyn QueueBackend>.
    let backend = &pool.backend;

    let job_id = backend
        .enqueue(&claims.sub, "agent_execution", payload, None)
        .await
        .map_err(|e| ApiError::Internal(format!("Queue error: {}", e)))?;

    // Record metrics
    state.metrics().record_llm_call(0, false); // Just counting requests for now

    Ok(Json(ExecuteResponse {
        agent_id,
        response: format!("Job queued: {}", job_id),
        verified: false,
        confidence: 1.0,
        context_hash: "pending".to_string(),
        latency_ms: start.elapsed().as_millis() as u64,
    }))
}

/// Metrics response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct MetricsResponse {
    pub llm_calls: u64,
    pub llm_errors: u64,
    pub tokens_used: u64,
    pub debates: u64,
    pub agents_created: u64,
    pub verifications: u64,
    pub verification_rate: f64,
    pub error_rate: f64,
}

/// Get metrics handler
#[utoipa::path(
    get,
    path = "/api/v1/metrics",
    responses(
        (status = 200, description = "Current system metrics", body = MetricsResponse),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn get_metrics(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> ApiResult<Json<MetricsResponse>> {
    // Only admins can view metrics
    if !claims.has_role("admin") {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let snapshot = state.metrics().snapshot();

    Ok(Json(MetricsResponse {
        llm_calls: snapshot.llm_calls,
        llm_errors: snapshot.llm_errors,
        tokens_used: snapshot.tokens_used,
        debates: snapshot.debates,
        agents_created: snapshot.agents_created,
        verifications: snapshot.verifications,
        verification_rate: state.metrics().verification_rate(),
        error_rate: state.metrics().llm_error_rate(),
    }))
}

/// Prometheus metrics handler
#[utoipa::path(
    get,
    path = "/metrics",
    responses(
        (status = 200, description = "Prometheus formatted metrics", body = String)
    )
)]
pub async fn get_prometheus_metrics(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> ApiResult<String> {
    // Only admins can view metrics
    if !claims.has_role("admin") {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let snapshot = state.metrics().snapshot();
    Ok(snapshot.to_prometheus())
}

#[derive(OpenApi)]
#[openapi(
    paths(
        health,
        health_detailed,
        create_agent,
        execute_agent,
        get_metrics,
        get_prometheus_metrics,
        crate::a2a::handler::agent_card_handler,
        crate::a2a::handler::create_task_handler,
        crate::a2a::handler::get_task_handler,
    ),
    components(
        schemas(
            HealthResponse, ComponentHealth, ComponentStatus,
            CreateAgentRequest, AgentResponse,
            ExecuteRequest, ExecuteResponse,
            MetricsResponse,
            crate::a2a::agent_card::AgentCard,
            crate::a2a::agent_card::AuthConfig,
            crate::a2a::agent_card::Skill,
            crate::a2a::task::TaskRequest,
            crate::a2a::task::TaskResponse,
            crate::a2a::task::TaskStatus,
        )
    ),
    modifiers(&SecurityAddon)
)]
pub struct ApiDoc;

struct SecurityAddon;

impl utoipa::Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        if let Some(components) = openapi.components.as_mut() {
            components.add_security_scheme(
                "jwt",
                utoipa::openapi::security::SecurityScheme::Http(
                    utoipa::openapi::security::HttpBuilder::new()
                        .scheme(utoipa::openapi::security::HttpAuthScheme::Bearer)
                        .bearer_format("JWT")
                        .build(),
                ),
            )
        }
    }
}

/// Build the API router
pub fn api_router(state: AppState) -> Router {
    use utoipa_swagger_ui::SwaggerUi;

    Router::new()
        // Documentation endpoints
        .merge(SwaggerUi::new("/swagger-ui").url("/api-docs/openapi.json", ApiDoc::openapi()))
        // A2A Protocol endpoints
        .merge(crate::a2a::handler::a2a_routes().with_state(state.a2a_state()))
        // Public endpoints
        .route("/health", get(health))
        .route("/health/detailed", get(health_detailed))
        // Agent endpoints
        .route("/api/v1/agents", post(create_agent))
        .route("/api/v1/agents/{id}/execute", post(execute_agent))
        // Admin endpoints
        .route("/api/v1/metrics", get(get_metrics))
        .route("/metrics", get(get_prometheus_metrics))
        // State
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_response() {
        let health = HealthResponse {
            status: "healthy".to_string(),
            version: "0.1.0".to_string(),
            timestamp: chrono::Utc::now(),
            components: None,
        };
        assert_eq!(health.status, "healthy");
    }
}
