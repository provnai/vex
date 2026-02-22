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
pub mod evolution_store;
pub mod queue;
pub mod sqlite;
pub mod vector_store;

pub use agent_store::AgentStore;
pub use api_key_store::{validate_api_key, ApiKeyError, ApiKeyRecord, ApiKeyStore};
pub use audit_store::AuditStore;
pub use backend::{StorageBackend, StorageError, StorageExt};
pub use context_store::ContextStore;
pub use evolution_store::{EvolutionStore, EvolutionStoreError, SqliteEvolutionStore};
pub use vector_store::{
    MemoryVectorStore, SqliteVectorStore, VectorEmbedding, VectorError, VectorStoreBackend,
};
