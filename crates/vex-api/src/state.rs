//! Application State
//!
//! Centralizes access to DB, Queue, and Auth/Monitoring services.

use crate::a2a::handler::A2aState;
use crate::auth::JwtAuth;
use crate::tenant_rate_limiter::TenantRateLimiter;
use std::sync::Arc;
use vex_llm::Metrics;
use vex_persist::StorageBackend;
use vex_queue::{QueueBackend, WorkerPool};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    jwt_auth: JwtAuth,
    rate_limiter: Arc<TenantRateLimiter>,
    metrics: Arc<Metrics>,
    db: Arc<dyn StorageBackend>,
    queue: Arc<WorkerPool<dyn QueueBackend>>,
    a2a_state: Arc<A2aState>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        jwt_auth: JwtAuth,
        rate_limiter: Arc<TenantRateLimiter>,
        metrics: Arc<Metrics>,
        db: Arc<dyn StorageBackend>,
        queue: Arc<WorkerPool<dyn QueueBackend>>,
        a2a_state: Arc<A2aState>,
    ) -> Self {
        Self {
            jwt_auth,
            rate_limiter,
            metrics,
            db,
            queue,
            a2a_state,
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

    /// Get queue worker pool (cloned Arc for sharing)
    pub fn queue(&self) -> Arc<WorkerPool<dyn QueueBackend>> {
        self.queue.clone()
    }

    /// Get A2A state (cloned Arc for sharing)
    pub fn a2a_state(&self) -> Arc<A2aState> {
        self.a2a_state.clone()
    }
}
