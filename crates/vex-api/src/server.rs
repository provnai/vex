//! VEX API Server with graceful shutdown

use axum::{middleware, Router};
use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tower_http::compression::CompressionLayer;

use crate::auth::JwtAuth;
use crate::error::ApiError;
use crate::middleware::{
    auth_middleware, body_limit_layer, cors_layer, rate_limit_middleware, request_id_middleware,
    timeout_layer, tracing_middleware,
};
use crate::routes::api_router;
use vex_llm::{Metrics, RateLimitConfig, RateLimiter};
// use vex_persist::StorageBackend; // Not dealing with trait directly here
// use vex_queue::WorkerPool;

/// TLS configuration for HTTPS
#[derive(Debug, Clone)]
pub struct TlsConfig {
    /// Path to certificate file (PEM format)
    pub cert_path: String,
    /// Path to private key file (PEM format)
    pub key_path: String,
}

impl TlsConfig {
    /// Create TLS config from paths
    pub fn new(cert_path: &str, key_path: &str) -> Self {
        Self {
            cert_path: cert_path.to_string(),
            key_path: key_path.to_string(),
        }
    }

    /// Create from environment variables VEX_TLS_CERT and VEX_TLS_KEY
    pub fn from_env() -> Option<Self> {
        let cert = std::env::var("VEX_TLS_CERT").ok()?;
        let key = std::env::var("VEX_TLS_KEY").ok()?;
        Some(Self::new(&cert, &key))
    }
}

/// Server configuration
#[derive(Debug, Clone)]
pub struct ServerConfig {
    /// Server address
    pub addr: SocketAddr,
    /// Request timeout
    pub timeout: Duration,
    /// Max request body size (bytes)
    pub max_body_size: usize,
    /// Enable compression
    pub compression: bool,
    /// Rate limit config
    pub rate_limit: RateLimitConfig,
    /// Optional TLS configuration for HTTPS
    pub tls: Option<TlsConfig>,
}

impl Default for ServerConfig {
    fn default() -> Self {
        Self {
            addr: "0.0.0.0:8080".parse().unwrap(),
            timeout: Duration::from_secs(30),
            max_body_size: 1024 * 1024, // 1MB
            compression: true,
            rate_limit: RateLimitConfig::default(),
            tls: None,
        }
    }
}

impl ServerConfig {
    /// Create from environment variables
    pub fn from_env() -> Self {
        let port: u16 = std::env::var("VEX_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(8080);

        let timeout_secs: u64 = std::env::var("VEX_TIMEOUT_SECS")
            .ok()
            .and_then(|t| t.parse().ok())
            .unwrap_or(30);

        Self {
            addr: SocketAddr::from(([0, 0, 0, 0], port)),
            timeout: Duration::from_secs(timeout_secs),
            ..Default::default()
        }
    }
}

use crate::state::AppState;

/// VEX API Server
pub struct VexServer {
    config: ServerConfig,
    app_state: AppState,
}

impl VexServer {
    /// Create a new server
    pub async fn new(config: ServerConfig) -> Result<Self, ApiError> {
        use crate::jobs::agent::{AgentExecutionJob, AgentJobPayload};
        use vex_llm::{DeepSeekProvider, LlmProvider, MockProvider};
        use vex_queue::{QueueBackend, WorkerConfig, WorkerPool};

        let jwt_auth = JwtAuth::from_env()?;
        let rate_limiter = Arc::new(RateLimiter::new(config.rate_limit.clone()));
        let metrics = Arc::new(Metrics::new());

        // Initialize Persistence (SQLite)
        let db_url =
            std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite::memory:".to_string());
        let db = vex_persist::sqlite::SqliteBackend::new(&db_url)
            .await
            .map_err(|e| ApiError::Internal(format!("DB Init failed: {}", e)))?;

        // Initialize Queue (Persistent SQLite)
        let queue_backend = vex_persist::queue::SqliteQueueBackend::new(db.pool().clone());

        // Use dynamic dispatch for the worker pool backend
        let worker_pool = WorkerPool::new_with_arc(
            Arc::new(queue_backend) as Arc<dyn QueueBackend>,
            WorkerConfig::default(),
        );

        // Initialize Intelligence (LLM)
        let llm: Arc<dyn LlmProvider> = if let Ok(key) = std::env::var("DEEPSEEK_API_KEY") {
            tracing::info!("Initializing DeepSeek Provider");
            Arc::new(DeepSeekProvider::chat(&key))
        } else {
            tracing::warn!("DEEPSEEK_API_KEY not found. Using Mock Provider.");
            Arc::new(MockProvider::smart())
        };

        // Create shared result store for job results
        let result_store = crate::jobs::new_result_store();

        // Register Agent Job
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

        let app_state = AppState::new(
            jwt_auth,
            rate_limiter,
            metrics,
            Arc::new(db),
            Arc::new(worker_pool),
        );

        Ok(Self { config, app_state })
    }

    /// Build the complete router with all middleware
    pub fn router(&self) -> Router {
        let mut app = api_router(self.app_state.clone());

        // Apply middleware layers (order matters - bottom to top execution)
        app = app
            // Compression (outermost - compresses response)
            .layer(CompressionLayer::new())
            // Body size limit
            .layer(body_limit_layer(self.config.max_body_size))
            // Timeout
            .layer(timeout_layer(self.config.timeout))
            // CORS
            .layer(cors_layer())
            // Request ID
            .layer(middleware::from_fn(request_id_middleware))
            // Tracing
            .layer(middleware::from_fn_with_state(
                self.app_state.clone(),
                tracing_middleware,
            ))
            // Rate limiting
            .layer(middleware::from_fn_with_state(
                self.app_state.clone(),
                rate_limit_middleware,
            ))
            // Authentication (innermost - runs first)
            .layer(middleware::from_fn_with_state(
                self.app_state.clone(),
                auth_middleware,
            ));

        app
    }

    /// Run the server with graceful shutdown
    pub async fn run(self) -> Result<(), ApiError> {
        let app = self.router();
        let addr = self.config.addr;

        tracing::info!("Starting VEX API server on {}", addr);

        // Start Worker Pool in background
        let queue = self.app_state.queue();
        tokio::spawn(async move {
            queue.start().await;
        });

        let listener = tokio::net::TcpListener::bind(addr)
            .await
            .map_err(|e| ApiError::Internal(format!("Failed to bind: {}", e)))?;

        axum::serve(listener, app)
            .with_graceful_shutdown(shutdown_signal())
            .await
            .map_err(|e| ApiError::Internal(format!("Server error: {}", e)))?;

        tracing::info!("Server shutdown complete");
        Ok(())
    }

    /// Get server metrics
    pub fn metrics(&self) -> Arc<Metrics> {
        self.app_state.metrics()
    }
}

/// Graceful shutdown signal handler
async fn shutdown_signal() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            tracing::info!("Received Ctrl+C, starting graceful shutdown");
        }
        _ = terminate => {
            tracing::info!("Received SIGTERM, starting graceful shutdown");
        }
    }
}

/// Initialize tracing subscriber
pub fn init_tracing() {
    use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("info,vex_api=debug,tower_http=debug"));

    tracing_subscriber::registry()
        .with(filter)
        .with(tracing_subscriber::fmt::layer().with_target(true))
        .init();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_default() {
        let config = ServerConfig::default();
        assert_eq!(config.addr.port(), 8080);
        assert_eq!(config.timeout, Duration::from_secs(30));
    }
}
