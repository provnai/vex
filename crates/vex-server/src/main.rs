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

// mod custom_queue; // Removed: Using patched library code in v0.1.8 instead

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
    let db = Arc::new(
        vex_persist::sqlite::SqliteBackend::new(&db_url)
            .await
            .map_err(|e| anyhow::anyhow!("DB Init failed: {}", e))?,
    );

    let evolution_store: Arc<dyn vex_persist::EvolutionStore> =
        Arc::new(vex_persist::SqliteEvolutionStore::new(db.pool().clone()));

    // Initialize Queue
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

    // Create FileAnchor for audit logging
    let file_anchor: Arc<dyn vex_anchor::AnchorBackend> =
        Arc::new(vex_anchor::FileAnchor::new("./vex_audit.jsonl"));
    tracing::info!("⚓ Audit logging enabled: ./vex_audit.jsonl");

    // Register Agent Job
    let llm_clone = llm.clone();
    let result_store_clone = result_store.clone();
    let db_for_factory = db.clone();
    let evolution_store_clone = evolution_store.clone();
    let gate: std::sync::Arc<dyn vex_runtime::Gate> = std::sync::Arc::new(vex_runtime::ChoraGateMock::default());
    let gate_clone = gate.clone();
    worker_pool.register_job_factory("agent_execution", move |payload| {
        let job_payload: AgentJobPayload =
            serde_json::from_value(payload).unwrap_or_else(|_| AgentJobPayload {
                agent_id: "unknown".to_string(),
                prompt: "payload error".to_string(),
                context_id: None,
                enable_adversarial: false,
                enable_self_correction: false,
                max_debate_rounds: 3,
                tenant_id: None,
            });
        let job_id = uuid::Uuid::new_v4();
        let anchor_clone = file_anchor.clone();
        let db_concrete = db_for_factory.clone();
        let evo_store = evolution_store_clone.clone();

        Box::new(AgentExecutionJob::new(
            job_id,
            job_payload,
            llm_clone.clone(),
            result_store_clone.clone(),
            db_concrete as Arc<dyn vex_persist::StorageBackend>,
            Some(anchor_clone),
            evo_store,
            gate_clone.clone(),
        ))
    });

    let a2a_state = Arc::new(vex_api::a2a::handler::A2aState::default());

    let app_state = AppState::new(
        jwt_auth,
        rate_limiter,
        metrics,
        db as Arc<dyn vex_persist::StorageBackend>,
        evolution_store,
        Arc::new(worker_pool),
        a2a_state,
        llm.clone(),
        None, // We skip the broken router entirely
        gate.clone(),
    );

    let mut app = api_router(app_state.clone());

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

// CHORA A/B Test Middleware REMOVED for Production v0.1.8 Release
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
