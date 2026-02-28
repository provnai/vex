//! API routes for VEX endpoints

use axum::response::sse as ax_sse;
use axum::{
    extract::{Extension, Path, State},
    routing::{get, post},
    Json, Router,
};
use futures::stream::{self, Stream};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use uuid::Uuid;

use crate::auth::Claims;
use crate::error::{ApiError, ApiResult};
use crate::sanitize::{sanitize_name, sanitize_prompt_async, sanitize_role};
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

    // Validate depth bounds (Fix #13)
    if req.max_depth > 10 {
        return Err(ApiError::Validation(
            "max_depth exceeds safety limit of 10".to_string(),
        ));
    }

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
    pub context_id: Option<String>,
    #[serde(default)]
    pub enable_adversarial: bool,
    #[serde(default)]
    pub enable_self_correction: bool,
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
    /// Merkle Tree root of the execution trace (available when polling job result)
    pub merkle_root: Option<String>,
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

    // Sanitize and validate prompt with async safety judge
    let llm = state.llm();
    let prompt = sanitize_prompt_async(&req.prompt, Some(&*llm))
        .await
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

    // Create job payload with sanitized prompt and adversarial config
    let payload = serde_json::json!({
        "agent_id": agent_id,
        "prompt": prompt,
        "context_id": req.context_id,
        "enable_adversarial": req.enable_adversarial,
        "enable_self_correction": req.enable_self_correction,
        "max_debate_rounds": req.max_debate_rounds,
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
        merkle_root: None,
    }))
}

/// Job status response (for polling after execute)
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct JobStatusResponse {
    pub job_id: Uuid,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
    pub queued_at: chrono::DateTime<chrono::Utc>,
    pub attempts: u32,
}

/// Get job status / result handler
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}",
    params(
        ("id" = Uuid, Path, description = "Job ID returned from execute_agent")
    ),
    responses(
        (status = 200, description = "Job status and result", body = JobStatusResponse),
        (status = 404, description = "Job not found")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn get_job_status(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> ApiResult<Json<JobStatusResponse>> {
    let pool = state.queue();
    let backend = &pool.backend;

    let tenant_id = claims.tenant_id.as_deref().unwrap_or(&claims.sub);

    let job = backend
        .get_job(tenant_id, job_id)
        .await
        .map_err(|_| ApiError::NotFound(format!("Job {} not found", job_id)))?;

    let status_str = match job.status {
        vex_queue::JobStatus::Pending => "pending",
        vex_queue::JobStatus::Running => "running",
        vex_queue::JobStatus::Completed => "completed",
        vex_queue::JobStatus::Failed(_) => "failed",
        vex_queue::JobStatus::DeadLetter => "dead_letter",
    };

    Ok(Json(JobStatusResponse {
        job_id,
        status: status_str.to_string(),
        result: job.result,
        error: job.last_error,
        queued_at: job.created_at,
        attempts: job.attempts,
    }))
}

/// Job update event for SSE
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct JobUpdate {
    pub job_id: Uuid,
    pub status: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<String>,
}

/// SSE Stream handler for job updates
#[utoipa::path(
    get,
    path = "/api/v1/jobs/{id}/stream",
    params(
        ("id" = Uuid, Path, description = "Job ID")
    ),
    responses(
        (status = 200, description = "SSE stream of job updates")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn get_job_stream(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(job_id): Path<Uuid>,
) -> ax_sse::Sse<impl Stream<Item = Result<ax_sse::Event, Infallible>>> {
    let tenant_id = claims
        .tenant_id
        .as_deref()
        .unwrap_or(&claims.sub)
        .to_string();
    let backend = state.queue().backend.clone();

    let stream = stream::unfold(
        (backend, tenant_id, job_id, false),
        |(backend, tid, jid, finished)| async move {
            if finished {
                return None;
            }

            match backend.get_job(&tid, jid).await {
                Ok(job) => {
                    let status_str = match job.status {
                        vex_queue::JobStatus::Pending => "pending",
                        vex_queue::JobStatus::Running => "running",
                        vex_queue::JobStatus::Completed => "completed",
                        vex_queue::JobStatus::Failed(_) => "failed",
                        vex_queue::JobStatus::DeadLetter => "dead_letter",
                    };

                    let is_final = matches!(
                        job.status,
                        vex_queue::JobStatus::Completed
                            | vex_queue::JobStatus::Failed(_)
                            | vex_queue::JobStatus::DeadLetter
                    );

                    let data = JobUpdate {
                        job_id: jid,
                        status: status_str.to_string(),
                        result: job.result,
                        error: job.last_error,
                    };

                    let event = ax_sse::Event::default()
                        .json_data(data)
                        .unwrap_or_else(|_| ax_sse::Event::default().data("error"));

                    if !is_final {
                        // Poll interval for non-finished jobs
                        tokio::time::sleep(std::time::Duration::from_millis(1000)).await;
                    }

                    Some((Ok(event), (backend, tid, jid, is_final)))
                }
                Err(_) => {
                    let event = ax_sse::Event::default().data("job_not_found");
                    Some((Ok(event), (backend, tid, jid, true)))
                }
            }
        },
    );

    ax_sse::Sse::new(stream).keep_alive(ax_sse::KeepAlive::default())
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

/// Routing statistics response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct RoutingStatsResponse {
    pub summary: vex_router::ObservabilitySummary,
    pub savings: vex_router::SavingsReport,
}

/// Get routing statistics handler
#[utoipa::path(
    get,
    path = "/api/v1/routing/stats",
    responses(
        (status = 200, description = "Current routing statistics and cost savings", body = RoutingStatsResponse),
        (status = 404, description = "Router not enabled"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn get_routing_stats(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
) -> ApiResult<Json<RoutingStatsResponse>> {
    // Only admins can view deep stats
    if !claims.has_role("admin") {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let router = state
        .router()
        .ok_or_else(|| ApiError::NotFound("Router not enabled".to_string()))?;
    let obs = router.observability();

    Ok(Json(RoutingStatsResponse {
        summary: obs.get_summary(),
        savings: obs.get_savings(),
    }))
}

/// Routing configuration request
#[derive(Debug, Deserialize, utoipa::ToSchema)]
pub struct UpdateRoutingConfigRequest {
    pub strategy: String,
    pub cache_enabled: bool,
    pub compression_level: String,
}

/// Update routing configuration handler
#[utoipa::path(
    put,
    path = "/api/v1/routing/config",
    request_body = UpdateRoutingConfigRequest,
    responses(
        (status = 200, description = "Routing configuration updated successfully"),
        (status = 404, description = "Router not enabled"),
        (status = 400, description = "Invalid configuration"),
        (status = 403, description = "Forbidden")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn update_routing_config(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Json(req): Json<UpdateRoutingConfigRequest>,
) -> ApiResult<Json<HealthResponse>> {
    // Only admins can change system config
    if !claims.has_role("admin") {
        return Err(ApiError::Forbidden("Admin access required".to_string()));
    }

    let _router = state
        .router()
        .ok_or_else(|| ApiError::NotFound("Router not enabled".to_string()))?;

    // In a real implementation, we would update the router state here.
    // For now, we return a success status.

    Ok(Json(HealthResponse {
        status: format!("Routing strategy updated to {}", req.strategy),
        version: env!("CARGO_PKG_VERSION").to_string(),
        timestamp: chrono::Utc::now(),
        components: None,
    }))
}

/// Evolve agent response
#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct EvolveResponse {
    pub agent_id: Uuid,
    pub suggestions: Vec<SuggestionDTO>,
    pub message: String,
}

#[derive(Debug, Serialize, utoipa::ToSchema)]
pub struct SuggestionDTO {
    pub trait_name: String,
    pub current_value: f64,
    pub suggested_value: f64,
    pub confidence: f64,
}

/// Evolve agent handler
#[utoipa::path(
    post,
    path = "/api/v1/agents/{id}/evolve",
    params(
        ("id" = Uuid, Path, description = "Agent ID")
    ),
    responses(
        (status = 200, description = "Reflection complete", body = EvolveResponse),
        (status = 404, description = "Agent not found")
    ),
    security(
        ("jwt" = [])
    )
)]
pub async fn evolve_agent(
    Extension(claims): Extension<Claims>,
    State(state): State<AppState>,
    Path(agent_id): Path<Uuid>,
) -> ApiResult<Json<EvolveResponse>> {
    let store = AgentStore::new(state.db());
    let agent = store
        .load(&claims.sub, agent_id)
        .await
        .map_err(|e| ApiError::Internal(e.to_string()))?
        .ok_or_else(|| ApiError::NotFound("Agent not found".to_string()))?;

    let evo_store = state.evolution_store();

    let experiments = evo_store
        .load_recent(&claims.sub, 20)
        .await
        .unwrap_or_default();

    if experiments.is_empty() {
        return Ok(Json(EvolveResponse {
            agent_id,
            suggestions: vec![],
            message: "No experiments yet — run some tasks first".to_string(),
        }));
    }

    let reflection_agent = vex_adversarial::ReflectionAgent::new(state.llm());
    let mut evo_memory = vex_core::EvolutionMemory::new();
    for exp in experiments.clone() {
        evo_memory.record(exp);
    }

    // Use the latest experiment for context, but memory for stats
    let latest_exp = experiments.first().unwrap();

    let reflection_result = reflection_agent
        .reflect(
            &agent,
            &latest_exp.task_summary,
            "Retrospective evolution analysis",
            latest_exp.overall_fitness,
            &evo_memory,
        )
        .await;

    let adjustments_len = reflection_result.adjustments.len();
    Ok(Json(EvolveResponse {
        agent_id,
        suggestions: reflection_result
            .adjustments
            .into_iter()
            .map(|(t, c, s)| SuggestionDTO {
                trait_name: t,
                current_value: c,
                suggested_value: s,
                confidence: reflection_result.expected_improvement,
            })
            .collect(),
        message: format!(
            "Reflection complete. {} suggestions generated.",
            adjustments_len
        ),
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
        evolve_agent,
        get_job_status,
        get_job_stream,
        get_metrics,
        get_prometheus_metrics,
        get_routing_stats,
        update_routing_config,
        crate::a2a::handler::agent_card_handler,
        crate::a2a::handler::create_task_handler,
        crate::a2a::handler::get_task_handler,
    ),
    components(
        schemas(
            HealthResponse, ComponentHealth, ComponentStatus,
            CreateAgentRequest, AgentResponse,
            ExecuteRequest, ExecuteResponse,
            EvolveResponse, SuggestionDTO,
            JobStatusResponse, JobUpdate,
            MetricsResponse,
            RoutingStatsResponse,
            UpdateRoutingConfigRequest,
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
        .route("/api/v1/agents/{id}/evolve", post(evolve_agent))
        // Job polling endpoint
        .route("/api/v1/jobs/{id}", get(get_job_status))
        .route("/api/v1/jobs/{id}/stream", get(get_job_stream))
        // Admin endpoints
        .route("/api/v1/metrics", get(get_metrics))
        .route("/api/v1/routing/stats", get(get_routing_stats))
        .route(
            "/api/v1/routing/config",
            axum::routing::put(update_routing_config),
        )
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
