//! Gateway - HTTP API Server with all enhanced features

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::trace::TraceLayer;

use crate::cache::SemanticCache;
use crate::classifier::QueryClassifier;
use crate::compress::{CompressionLevel, PromptCompressor};
use crate::config::{Config, RoutingStrategy};
use crate::guardrails::Guardrails;
use crate::models::ModelPool;
use crate::observability::{Observability, ObservabilitySummary, RequestMetrics, SavingsReport};
use crate::router::{Router as RoutingEngine, RouterConfig, RoutingDecision};

/// Application state with all new features
pub struct AppState {
    pub config: Config,
    pub pool: ModelPool,
    pub classifier: QueryClassifier,
    pub engine: crate::router::Router,
    pub cache: SemanticCache,
    pub compressor: PromptCompressor,
    pub guardrails: Guardrails,
    pub observability: Observability,
}

/// Main server
pub struct Server {
    state: Arc<AppState>,
}

impl Server {
    pub fn new(config: Config) -> Self {
        let pool = ModelPool::new(config.models.clone());
        let classifier = QueryClassifier::new();
        let engine = crate::router::Router::new();

        let cache = SemanticCache::new(0.85, config.cache_enabled as usize * 10000, 86400);

        let compressor = PromptCompressor::new(CompressionLevel::Balanced);
        let guardrails = Guardrails::new(true);
        let observability = Observability::new(10000);

        let state = Arc::new(AppState {
            config,
            pool,
            classifier,
            engine,
            cache,
            compressor,
            guardrails,
            observability,
        });

        Self { state }
    }

    pub async fn run(&self) -> Result<(), Box<dyn std::error::Error>> {
        let app = Router::new()
            .route("/v1/chat/completions", post(chat_completions))
            .route("/v1/route", post(route_query))
            .route("/v1/feedback", post(submit_feedback))
            .route("/v1/cache/stats", get(cache_stats))
            .route("/v1/cache/clear", post(clear_cache))
            .route("/v1/analytics/summary", get(analytics_summary))
            .route("/v1/analytics/savings", get(analytics_savings))
            .route("/v1/analytics/costs", get(analytics_costs))
            .route("/health", get(health_check))
            .layer(
                ServiceBuilder::new()
                    .layer(TraceLayer::new_for_http())
                    .into_inner(),
            )
            .with_state(self.state.clone());

        let addr = format!(
            "{}:{}",
            self.state.config.server.host, self.state.config.server.port
        );

        println!("🔀 VEX Router starting on http://{}", addr);
        println!("📦 Features: Semantic Cache, Prompt Compression, Guardrails, Observability");

        let listener = tokio::net::TcpListener::bind(&addr).await?;
        axum::serve(listener, app).await?;

        Ok(())
    }
}

// ============================================================================
// API Handlers
// ============================================================================

#[derive(Debug, Deserialize)]
pub struct ChatRequest {
    pub messages: Vec<serde_json::Value>,
    pub routing: Option<serde_json::Value>,
    pub cache: Option<bool>,
    pub compression: Option<String>,
    pub guardrails: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct ChatResponse {
    pub id: String,
    pub object: String,
    pub created: u64,
    pub model: String,
    pub choices: Vec<serde_json::Value>,
    pub usage: serde_json::Value,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub smartrouter: Option<serde_json::Value>,
}

async fn chat_completions(
    State(state): State<Arc<AppState>>,
    Json(request): Json<ChatRequest>,
) -> Result<Json<ChatResponse>, StatusCode> {
    let request_id = uuid::Uuid::new_v4().to_string();
    let start_time = std::time::Instant::now();

    let enable_cache = request.cache.unwrap_or(true);
    let enable_guardrails = request.guardrails.unwrap_or(true);
    let compression_level = match request.compression.as_deref() {
        Some("none") => CompressionLevel::None,
        Some("light") => CompressionLevel::Light,
        Some("aggressive") => CompressionLevel::Aggressive,
        _ => CompressionLevel::Balanced,
    };

    let compressor = PromptCompressor::new(compression_level);

    let query_text: String = request
        .messages
        .iter()
        .filter_map(|m| m.get("content").and_then(|c| c.as_str()))
        .collect::<Vec<_>>()
        .join(" ");

    if enable_guardrails {
        let guard_result = state.guardrails.check_input(&query_text);
        if !guard_result.passed {
            return Err(StatusCode::BAD_REQUEST);
        }
    }

    let compressed = compressor.compress(&query_text);
    let processed_query = if compressed.compression_ratio > 0.0 {
        compressed.compressed.clone()
    } else {
        query_text.clone()
    };

    let mut cache_hit = false;
    let mut cache_similarity = None;

    if enable_cache {
        if let Some(cached) = state.cache.get(&processed_query) {
            cache_hit = true;
            cache_similarity = Some(cached.similarity);

            let latency = start_time.elapsed().as_millis() as u64;
            let cost = calculate_request_cost(&state.pool, "gpt-4o-mini", 50, 50);

            let metrics = build_metrics(
                &request_id,
                "gpt-4o-mini",
                "auto",
                0.1,
                50,
                50,
                cost,
                latency,
                cache_hit,
                cache_similarity,
                compressed.compression_ratio,
                true,
                None,
            );
            state.observability.record(metrics);

            return Ok(Json(build_response(
                request_id,
                "gpt-4o-mini".to_string(),
                cached.response,
                50,
                50,
                Some(build_smartrouter_info(
                    "semantic_cache_hit",
                    0.1,
                    95.0,
                    cache_hit,
                    cache_similarity,
                    compressed.compression_ratio,
                    latency,
                    true,
                )),
            )));
        }
    }

    let complexity = state.classifier.classify(&processed_query);

    let decision = state
        .engine
        .route(&processed_query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response_text = format!(
        "[VEX Router: {}] Query complexity: {:.2}, Savings: {:.0}%\n\nProcessed: {}",
        decision.model_id,
        0.5,
        decision.estimated_savings,
        if query_text != processed_query {
            format!("(compressed from {} tokens) ", compressed.original_tokens)
        } else {
            String::new()
        }
    );

    if enable_cache {
        state.cache.store(
            &processed_query,
            response_text.clone(),
            compressed.compressed_tokens + 50,
        );
    }

    let latency = start_time.elapsed().as_millis() as u64;
    let cost = calculate_request_cost(
        &state.pool,
        &decision.model_id,
        compressed.compressed_tokens,
        50,
    );

    let metrics = build_metrics(
        &request_id,
        &decision.model_id,
        "auto",
        complexity.score,
        compressed.compressed_tokens,
        50,
        cost,
        latency,
        cache_hit,
        cache_similarity,
        compressed.compression_ratio,
        true,
        None,
    );
    state.observability.record(metrics);

    let smartrouter_info = build_smartrouter_info(
        &decision.reason,
        complexity.score,
        decision.estimated_savings,
        cache_hit,
        cache_similarity,
        compressed.compression_ratio,
        latency,
        true,
    );

    Ok(Json(build_response(
        request_id,
        decision.model_id,
        response_text,
        compressed.compressed_tokens,
        50,
        Some(smartrouter_info),
    )))
}

#[derive(Debug, Deserialize)]
pub struct RouteRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct RouteResponse {
    pub decision: serde_json::Value,
    pub complexity: serde_json::Value,
}

async fn route_query(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RouteRequest>,
) -> Result<Json<RouteResponse>, StatusCode> {
    let complexity = state.classifier.classify(&request.query);
    let decision = state
        .engine
        .route(&request.query)
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let response = RouteResponse {
        decision: serde_json::to_value(decision).map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?,
        complexity: serde_json::json!({
            "score": complexity.score,
            "estimated_tokens": complexity.estimated_tokens,
            "confidence": complexity.confidence
        }),
    };

    Ok(Json(response))
}

#[derive(Debug, Deserialize)]
pub struct FeedbackRequest {
    pub request_id: String,
    pub quality_score: Option<f64>,
    pub accepted: Option<bool>,
    pub feedback_text: Option<String>,
}

async fn submit_feedback(
    State(_state): State<Arc<AppState>>,
    Json(_request): Json<FeedbackRequest>,
) -> Result<Json<serde_json::Value>, StatusCode> {
    Ok(Json(serde_json::json!({
        "status": "feedback_received"
    })))
}

async fn cache_stats(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let stats = state.cache.stats();
    Json(serde_json::json!({
        "total_entries": stats.total_entries,
        "valid_entries": stats.valid_entries,
        "cache_size_bytes": stats.cache_size_bytes,
    }))
}

async fn clear_cache(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    state.cache.clear();
    Json(serde_json::json!({
        "status": "cache_cleared"
    }))
}

async fn analytics_summary(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let summary = state.observability.get_summary();
    Json(serde_json::to_value(summary).unwrap())
}

async fn analytics_savings(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let savings = state.observability.get_savings();
    Json(serde_json::to_value(savings).unwrap())
}

async fn analytics_costs(State(state): State<Arc<AppState>>) -> Json<serde_json::Value> {
    let costs = state.observability.get_cost_by_model();
    Json(serde_json::to_value(costs).unwrap())
}

async fn health_check() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "healthy",
        "service": "VEX Router",
        "version": "0.1.0",
        "features": [
            "semantic_cache",
            "prompt_compression",
            "guardrails",
            "observability"
        ]
    }))
}

// ============================================================================
// Helper Functions
// ============================================================================

fn build_response(
    id: String,
    model: String,
    content: String,
    input_tokens: u32,
    output_tokens: u32,
    smartrouter: Option<serde_json::Value>,
) -> ChatResponse {
    ChatResponse {
        id: format!("chatcmpl-{}", id),
        object: "chat.completion".to_string(),
        created: chrono::Utc::now().timestamp() as u64,
        model,
        choices: vec![serde_json::json!({
            "index": 0,
            "message": {
                "role": "assistant",
                "content": content
            },
            "finish_reason": "stop"
        })],
        usage: serde_json::json!({
            "prompt_tokens": input_tokens,
            "completion_tokens": output_tokens,
            "total_tokens": input_tokens + output_tokens
        }),
        smartrouter,
    }
}

fn build_smartrouter_info(
    reason: &str,
    complexity_score: f64,
    estimated_savings: f64,
    cache_hit: bool,
    cache_similarity: Option<f32>,
    compression_ratio: f64,
    latency_ms: u64,
    guardrails_passed: bool,
) -> serde_json::Value {
    let mut routing_reason = reason.to_string();
    if cache_hit {
        routing_reason = format!("{} + semantic_cache_hit", routing_reason);
    }

    serde_json::json!({
        "model_used": "gpt-4o-mini",
        "routing_reason": routing_reason,
        "complexity_score": complexity_score,
        "estimated_savings": format!("{:.0}%", estimated_savings),
        "cache_hit": cache_hit,
        "cache_similarity": cache_similarity,
        "compression_ratio": compression_ratio,
        "guardrails_passed": guardrails_passed,
        "latency_ms": latency_ms,
    })
}

fn build_metrics(
    request_id: &str,
    model_used: &str,
    routing_strategy: &str,
    complexity_score: f64,
    tokens_input: u32,
    tokens_output: u32,
    cost_usd: f64,
    latency_ms: u64,
    cache_hit: bool,
    cache_similarity: Option<f32>,
    compression_ratio: f64,
    guardrails_passed: bool,
    error: Option<String>,
) -> RequestMetrics {
    RequestMetrics {
        request_id: request_id.to_string(),
        timestamp: chrono::Utc::now().timestamp(),
        model_used: model_used.to_string(),
        routing_strategy: routing_strategy.to_string(),
        complexity_score,
        tokens_input,
        tokens_output,
        cost_usd,
        latency_ms,
        first_token_ms: None,
        cache_hit,
        cache_similarity,
        compression_ratio: if compression_ratio > 0.0 {
            Some(compression_ratio)
        } else {
            None
        },
        guardrails_passed,
        error,
    }
}

fn calculate_request_cost(
    pool: &ModelPool,
    model_id: &str,
    input_tokens: u32,
    output_tokens: u32,
) -> f64 {
    if let Some(model) = pool.get(model_id) {
        let input_cost = input_tokens as f64 * model.config.input_cost / 1_000_000.0;
        let output_cost = output_tokens as f64 * model.config.output_cost / 1_000_000.0;
        input_cost + output_cost
    } else {
        input_tokens as f64 * 0.60 / 1_000_000.0 + output_tokens as f64 * 0.60 / 1_000_000.0
    }
}
