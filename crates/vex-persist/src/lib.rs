//! # VEX Persistence
//!
//! Storage backends for agent state, context, and audit logs.
//!
//! Supports:
//! - In-memory (for testing)
//! - SQLite (for single-node)
//! - PostgreSQL (for production)

pub mod agent_store;
pub mod api_key_store;
pub mod audit_store;
pub mod backend;
pub mod context_store;
pub mod queue;
pub mod sqlite;

pub use agent_store::AgentStore;
pub use api_key_store::{ApiKeyError, ApiKeyRecord, ApiKeyStore, validate_api_key};
pub use audit_store::AuditStore;
pub use backend::{StorageBackend, StorageError, StorageExt};
pub use context_store::ContextStore;
