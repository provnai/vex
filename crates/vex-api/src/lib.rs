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

pub mod server;
pub mod routes;
pub mod middleware;
pub mod auth;
pub mod error;
pub mod circuit_breaker;
pub mod state;
pub mod jobs;
pub mod sanitize;
pub mod telemetry;

pub use server::{VexServer, ServerConfig};
pub use error::{ApiError, ApiResult};
pub use auth::{Claims, JwtAuth};
