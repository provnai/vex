//! Application State
//!
//! Centralizes access to DB, Queue, and Auth/Monitoring services.

use crate::auth::JwtAuth;
use std::sync::Arc;
use vex_llm::{Metrics, RateLimiter};
use vex_persist::StorageBackend;
use vex_queue::{QueueBackend, WorkerPool};

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    jwt_auth: JwtAuth,
    rate_limiter: Arc<RateLimiter>,
    metrics: Arc<Metrics>,
    db: Arc<dyn StorageBackend>,
    queue: Arc<WorkerPool<dyn QueueBackend>>,
}

impl AppState {
    /// Create new application state
    pub fn new(
        jwt_auth: JwtAuth,
        rate_limiter: Arc<RateLimiter>,
        metrics: Arc<Metrics>,
        db: Arc<dyn StorageBackend>,
        queue: Arc<WorkerPool<dyn QueueBackend>>,
    ) -> Self {
        Self {
            jwt_auth,
            rate_limiter,
            metrics,
            db,
            queue,
        }
    }

    /// Get JWT auth service
    pub fn jwt_auth(&self) -> &JwtAuth {
        &self.jwt_auth
    }

    /// Get rate limiter (cloned Arc for sharing)
    pub fn rate_limiter(&self) -> Arc<RateLimiter> {
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
}
