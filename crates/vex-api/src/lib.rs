//! # VEX API
//!
//! Industry-grade HTTP API gateway for VEX Protocol.
//!
//! Features:
//! - Axum-based web server
//! - Tower middleware (auth, rate limit, tracing)
//! - Circuit breaker pattern
//! - OpenTelemetry-ready observability
//! - JWT authentication
//! - Graceful shutdown

pub mod auth;
pub mod circuit_breaker;
pub mod error;
pub mod jobs;
pub mod middleware;
pub mod routes;
pub mod sanitize;
pub mod server;
pub mod state;
pub mod telemetry;

pub use auth::{Claims, JwtAuth};
pub use error::{ApiError, ApiResult};
pub use server::{ServerConfig, VexServer};
