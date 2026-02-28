//! Application State
//!
//! Centralizes access to DB, Queue, and Auth/Monitoring services.

use crate::a2a::handler::A2aState;
use crate::auth::JwtAuth;
use crate::tenant_rate_limiter::TenantRateLimiter;
use std::sync::Arc;
use vex_llm::{LlmProvider, Metrics};
use vex_persist::StorageBackend;
use vex_queue::{QueueBackend, WorkerPool};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    jwt_auth: JwtAuth,
    rate_limiter: Arc<TenantRateLimiter>,
    metrics: Arc<Metrics>,
    db: Arc<dyn StorageBackend>,
    evolution_store: Arc<dyn vex_persist::EvolutionStore>,
    queue: Arc<WorkerPool<dyn QueueBackend>>,
    a2a_state: Arc<A2aState>,
    llm: Arc<dyn LlmProvider>,
    router: Option<Arc<vex_router::Router>>,
}

impl AppState {
    /// Create new application state
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        jwt_auth: JwtAuth,
        rate_limiter: Arc<TenantRateLimiter>,
        metrics: Arc<Metrics>,
        db: Arc<dyn StorageBackend>,
        evolution_store: Arc<dyn vex_persist::EvolutionStore>,
        queue: Arc<WorkerPool<dyn QueueBackend>>,
        a2a_state: Arc<A2aState>,
        llm: Arc<dyn LlmProvider>,
        router: Option<Arc<vex_router::Router>>,
    ) -> Self {
        Self {
            jwt_auth,
            rate_limiter,
            metrics,
            db,
            evolution_store,
            queue,
            a2a_state,
            llm,
            router,
        }
    }

    /// Get JWT auth service
    pub fn jwt_auth(&self) -> &JwtAuth {
        &self.jwt_auth
    }

    /// Get rate limiter (cloned Arc for sharing)
    pub fn rate_limiter(&self) -> Arc<TenantRateLimiter> {
        self.rate_limiter.clone()
    }

    /// Get metrics collector (cloned Arc for sharing)
    pub fn metrics(&self) -> Arc<Metrics> {
        self.metrics.clone()
    }

    /// Get database backend (cloned Arc for sharing)
    pub fn db(&self) -> Arc<dyn StorageBackend> {
        self.db.clone()
    }

    /// Get evolution store (cloned Arc for sharing)
    pub fn evolution_store(&self) -> Arc<dyn vex_persist::EvolutionStore> {
        self.evolution_store.clone()
    }

    /// Get queue worker pool (cloned Arc for sharing)
    pub fn queue(&self) -> Arc<WorkerPool<dyn QueueBackend>> {
        self.queue.clone()
    }

    /// Get A2A state (cloned Arc for sharing)
    pub fn a2a_state(&self) -> Arc<A2aState> {
        self.a2a_state.clone()
    }

    /// Get LLM provider (cloned Arc for sharing)
    pub fn llm(&self) -> Arc<dyn LlmProvider> {
        self.llm.clone()
    }

    /// Get Router specifically (cloned Arc for sharing)
    pub fn router(&self) -> Option<Arc<vex_router::Router>> {
        self.router.clone()
    }
}
