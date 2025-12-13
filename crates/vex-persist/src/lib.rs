//! # VEX Persistence
//!
//! Storage backends for agent state, context, and audit logs.
//!
//! Supports:
//! - In-memory (for testing)
//! - SQLite (for single-node)
//! - PostgreSQL (for production)

pub mod backend;
pub mod agent_store;
pub mod context_store;
pub mod audit_store;
pub mod api_key_store;
pub mod sqlite;
pub mod queue;

pub use backend::{StorageBackend, StorageExt, StorageError};
pub use agent_store::AgentStore;
pub use context_store::ContextStore;
pub use audit_store::AuditStore;
