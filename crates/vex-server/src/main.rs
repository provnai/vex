//! VEX Server - Standalone entry point for the VEX Protocol API
//!
//! This crate serves as a thin wrapper around `vex-api` to provide
//! a runnable binary for production deployments without modifying
//! the core library crate.

use anyhow::Result;
use axum::middleware;
use std::sync::Arc;
use tower_http::compression::CompressionLayer;
use vex_api::jobs::agent::{AgentExecutionJob, AgentJobPayload};
use vex_api::middleware::{
    auth_middleware, body_limit_layer, cors_layer, rate_limit_middleware, request_id_middleware,
    security_headers_middleware, timeout_layer, tracing_middleware,
};
use vex_api::routes::api_router;
use vex_api::state::AppState;
use vex_api::ServerConfig;
use vex_llm::{
    CachedProvider, DeepSeekProvider, LlmProvider, Metrics, MockProvider, OpenAIProvider,
    ResilientProvider,
};
use vex_queue::{QueueBackend, WorkerConfig, WorkerPool};

// ── CHORA A/B Test Imports ────────────────────────────────────────────────
use axum::{
    body::Body,
    extract::Request,
    http::{Method, StatusCode},
    response::Response,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
// ─────────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing using the standard configuration from vex-api
    vex_api::server::init_tracing();

    tracing::info!("🚀 Starting VEX Protocol Server (Real Intelligence Wrapper)...");

    // Railway Compatibility: Map Railway's $PORT to VEX_PORT
    if let Ok(port) = std::env::var("PORT") {
        if std::env::var("VEX_PORT").is_err() {
            tracing::info!("Railway detected: Mapping PORT {} to VEX_PORT", port);
            std::env::set_var("VEX_PORT", port);
        }
    }

    // Railway Compatibility: Ensure VEX_JWT_SECRET exists to prevent startup crash.
    if std::env::var("VEX_JWT_SECRET").is_err() {
        tracing::warn!("VEX_JWT_SECRET not found! Using a temporary fallback secret.");
        std::env::set_var(
            "VEX_JWT_SECRET",
            "railway-default-fallback-secret-32-chars-long",
        );
    }

    // Load server configuration
    let config = ServerConfig::from_env();
    let jwt_auth = vex_api::auth::JwtAuth::from_env()
        .map_err(|e| anyhow::anyhow!("JWT Init failed: {}", e))?;
    let rate_limiter = Arc::new(vex_api::tenant_rate_limiter::TenantRateLimiter::new(
        vex_api::tenant_rate_limiter::RateLimitTier::Standard,
    ));
    let metrics = Arc::new(Metrics::new());

    // Initialize Persistence (SQLite)
    let db_url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
    let db = vex_persist::sqlite::SqliteBackend::new(&db_url)
        .await
        .map_err(|e| anyhow::anyhow!("DB Init failed: {}", e))?;

    // Initialize Queue (Persistent SQLite)
    let queue_backend = vex_persist::queue::SqliteQueueBackend::new(db.pool().clone());

    // Use dynamic dispatch for the worker pool backend
    let worker_pool = WorkerPool::new_with_arc(
        Arc::new(queue_backend) as Arc<dyn QueueBackend>,
        WorkerConfig::default(),
    );

    // REAL Intelligence Layer: Bypass the broken router!
    let llm: Arc<dyn LlmProvider> = if let Ok(key) = std::env::var("GROQ_API_KEY") {
        tracing::info!("Initializing Real Groq Provider (Fast+Free)");
        let base = GroqProvider::new(&key, "llama-3.3-70b-versatile");
        let resilient = ResilientProvider::new(base, vex_llm::LlmCircuitConfig::conservative());
        Arc::new(CachedProvider::wrap(resilient))
    } else if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
        tracing::info!("Initializing Real DeepSeek Provider");
        let base = DeepSeekProvider::chat(&key);
        let resilient = ResilientProvider::new(base, vex_llm::LlmCircuitConfig::conservative());
        Arc::new(CachedProvider::wrap(resilient))
    } else if let Ok(key) = std::env::var("OPENAI_API_KEY") {
        tracing::info!("Initializing Real OpenAI Provider (GPT-4)");
        let base = OpenAIProvider::gpt4(&key);
        let resilient = ResilientProvider::new(base, vex_llm::LlmCircuitConfig::conservative());
        Arc::new(CachedProvider::wrap(resilient))
    } else {
        tracing::warn!("No LLM API keys found. Falling back to Mock Provider.");
        Arc::new(MockProvider::smart())
    };

    // Create shared result store
    let result_store = vex_api::jobs::new_result_store();

    // Register Agent Job with the REAL LLM
    let llm_clone = llm.clone();
    let result_store_clone = result_store.clone();
    worker_pool.register_job_factory("agent_execution", move |payload| {
        let job_payload: AgentJobPayload =
            serde_json::from_value(payload).unwrap_or_else(|_| AgentJobPayload {
                agent_id: "unknown".to_string(),
                prompt: "payload error".to_string(),
                context_id: None,
            });
        let job_id = uuid::Uuid::new_v4();
        Box::new(AgentExecutionJob::new(
            job_id,
            job_payload,
            llm_clone.clone(),
            result_store_clone.clone(),
        ))
    });

    let a2a_state = Arc::new(vex_api::a2a::handler::A2aState::default());

    let app_state = AppState::new(
        jwt_auth,
        rate_limiter,
        metrics,
        Arc::new(db),
        Arc::new(worker_pool),
        a2a_state,
        llm.clone(),
        None, // We skip the broken router entirely
    );

    // Build the router with middleware
    // ── CHORA Research Layer (see RESEARCH.md for removal guide) ─────────
    // See: RESEARCH.md for full details, owner, and removal steps.
    // Enable with env var: VEX_CHORA_RESEARCH_MODE=true
    // Remove after CHORA A/B test data is collected.
    //
    // IMPLEMENTATION NOTE: We use a middleware layer (not a duplicate route)
    // to avoid Axum's startup panic when two routers register the same path.
    // The middleware intercepts requests before the standard handler sees them.
    let chora_research_enabled = std::env::var("VEX_CHORA_RESEARCH_MODE")
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if chora_research_enabled {
        tracing::warn!(
            "🔬 CHORA Research Mode ACTIVE — X-VEX-Bypass-Gate header enabled. See RESEARCH.md."
        );
    }

    let app_state_for_mw = app_state.clone();
    let mut app = api_router(app_state.clone()).layer(middleware::from_fn(
        move |req: Request<Body>, next: axum::middleware::Next| {
            let state = app_state_for_mw.clone();
            let enabled = chora_research_enabled;
            async move { chora_intercept(req, next, state, enabled).await }
        },
    ));

    app = app
        .layer(CompressionLayer::new())
        .layer(body_limit_layer(config.max_body_size))
        .layer(timeout_layer(config.timeout))
        .layer(cors_layer())
        .layer(middleware::from_fn(request_id_middleware))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            tracing_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            rate_limit_middleware,
        ))
        .layer(middleware::from_fn_with_state(
            app_state.clone(),
            auth_middleware,
        ))
        .layer(middleware::from_fn(security_headers_middleware));

    // Run the server
    tracing::info!("VEX API listening on {}", config.addr);

    // Start Worker Pool
    let queue = app_state.queue();
    tokio::spawn(async move {
        queue.start().await;
    });

    let listener = tokio::net::TcpListener::bind(config.addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    Ok(())
}

// ── CHORA A/B Test Middleware ─────────────────────────────────────────────
//
// RESEARCH ONLY — Not for production use. See RESEARCH.md for removal steps.
// Intercepts POST /api/v1/agents/{id}/execute before the standard handler.
// When X-VEX-Bypass-Gate: true is present AND research mode is enabled,
// it short-circuits the standard VEX Gate (AdvancedSanitizer) entirely.
// This avoids registering a duplicate route (which panics in Axum).
//
// Runs A: Standard request → Gate ON (normal handler)
// Runs B: X-VEX-Bypass-Gate: true → Gate OFF (enqueue raw prompt)
// ─────────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ChoraExecuteRequest {
    pub prompt: String,
}

#[derive(Debug, Serialize)]
struct ChoraExecuteResponse {
    pub agent_id: Uuid,
    pub response: String,
    pub gate_bypassed: bool,
    pub verified: bool,
    pub confidence: f64,
    pub context_hash: String,
    pub latency_ms: u64,
}

async fn chora_intercept(
    req: Request<Body>,
    next: axum::middleware::Next,
    state: AppState,
    enabled: bool,
) -> Response {
    // Only intercept POST /api/v1/agents/{id}/execute with the bypass header
    let is_execute = req.method() == Method::POST
        && req.uri().path().starts_with("/api/v1/agents/")
        && req.uri().path().ends_with("/execute");

    let bypass_header = req
        .headers()
        .get("X-VEX-Bypass-Gate")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    if !enabled || !is_execute || !bypass_header {
        // Not a CHORA bypass request — pass straight through to standard handler
        return next.run(req).await;
    }

    // Extract agent_id from path: /api/v1/agents/{id}/execute
    let path = req.uri().path().to_string();
    let agent_id_str = path
        .strip_prefix("/api/v1/agents/")
        .and_then(|s| s.strip_suffix("/execute"))
        .unwrap_or("unknown");
    let agent_id = Uuid::parse_str(agent_id_str).unwrap_or_else(|_| Uuid::nil());

    tracing::warn!(
        agent_id = %agent_id,
        "⚠️  CHORA A/B TEST: VEX Gate BYPASSED — raw prompt injected without sanitization"
    );

    // Extract tenant_id from JWT early, before req is consumed
    let auth_header = req
        .headers()
        .get("Authorization")
        .and_then(|h| h.to_str().ok());

    let tenant_id = if let Some(header) = auth_header {
        if let Ok(token) = vex_api::auth::JwtAuth::extract_from_header(header) {
            if let Ok(claims) = state.jwt_auth().decode(token) {
                Some(claims.tenant_id.unwrap_or(claims.sub))
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let tenant_id = tenant_id.unwrap_or_else(|| "chora-researcher".to_string());

    // Consume the body
    let start = std::time::Instant::now();
    let bytes = match axum::body::to_bytes(req.into_body(), 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Failed to read body"))
                .unwrap();
        }
    };

    let chora_req: ChoraExecuteRequest = match serde_json::from_slice(&bytes) {
        Ok(r) => r,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::UNPROCESSABLE_ENTITY)
                .body(Body::from(format!("Invalid JSON: {}", e)))
                .unwrap();
        }
    };

    // Enqueue directly — no sanitization
    // MUST match `AgentJobPayload` struct in `vex-api::jobs::agent`
    let payload = serde_json::json!({
        "agent_id": agent_id.to_string(),
        "prompt": chora_req.prompt,
        "context_id": None::<String>
    });

    let pool = state.queue();
    let backend = &pool.backend;

    let job_id = match backend
        .enqueue(&tenant_id, "agent_execution", payload, None)
        .await
    {
        Ok(id) => id,
        Err(e) => {
            return Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .body(Body::from(format!("Queue error: {}", e)))
                .unwrap();
        }
    };

    tracing::info!(agent_id = %agent_id, job_id = %job_id, "CHORA A/B bypass job enqueued");

    let resp = ChoraExecuteResponse {
        agent_id,
        response: format!("Job queued: {}", job_id),
        gate_bypassed: true,
        verified: false,
        confidence: 0.0,
        context_hash: "pending".to_string(),
        latency_ms: start.elapsed().as_millis() as u64,
    };

    Response::builder()
        .status(StatusCode::OK)
        .header("Content-Type", "application/json")
        .body(Body::from(serde_json::to_string(&resp).unwrap()))
        .unwrap()
}
// ─────────────────────────────────────────────────────────────────────────

async fn shutdown_signal() {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

/// Custom Groq Provider implemented directly in the wrapper to preserve library purity.
/// Groq is OpenAI-compatible but requires a custom base URL.
#[derive(Debug)]
struct GroqProvider {
    client: reqwest::Client,
    api_key: String,
    model: String,
}

impl GroqProvider {
    fn new(api_key: &str, model: &str) -> Self {
        Self {
            client: reqwest::Client::new(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl vex_llm::LlmProvider for GroqProvider {
    fn name(&self) -> &str {
        "groq"
    }

    async fn is_available(&self) -> bool {
        // Simple connectivity check
        self.client
            .get("https://api.groq.com/openai/v1/models")
            .bearer_auth(&self.api_key)
            .send()
            .await
            .is_ok()
    }

    async fn complete(
        &self,
        request: vex_llm::LlmRequest,
    ) -> Result<vex_llm::LlmResponse, vex_llm::LlmError> {
        let start = std::time::Instant::now();

        let response = self
            .client
            .post("https://api.groq.com/openai/v1/chat/completions")
            .bearer_auth(&self.api_key)
            .json(&serde_json::json!({
                "model": self.model,
                "messages": [
                    {"role": "system", "content": request.system},
                    {"role": "user", "content": request.prompt}
                ],
                "temperature": request.temperature,
            }))
            .send()
            .await
            .map_err(|e| vex_llm::LlmError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let err_text = response.text().await.unwrap_or_default();
            return Err(vex_llm::LlmError::RequestFailed(err_text));
        }

        let json: serde_json::Value = response
            .json()
            .await
            .map_err(|e| vex_llm::LlmError::InvalidResponse(e.to_string()))?;

        let content = json["choices"][0]["message"]["content"]
            .as_str()
            .ok_or_else(|| vex_llm::LlmError::InvalidResponse("Missing content".to_string()))?
            .to_string();

        let tokens = json["usage"]["total_tokens"].as_u64().map(|t| t as u32);

        Ok(vex_llm::LlmResponse {
            content,
            model: self.model.clone(),
            tokens_used: tokens,
            latency_ms: start.elapsed().as_millis() as u64,
            trace_root: None,
        })
    }
}
