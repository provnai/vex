//! # VEX Persistence
//!
//! Storage backends for agent state, context, and audit logs.
//!
//! Supports:
//! - In-memory (for testing)
//! - SQLite (for single-node)
//! - PostgreSQL (for production / Railway multi-node)

pub mod agent_store;
pub mod api_key_store;
pub mod audit_store;
pub mod backend;
pub mod context_store;
pub mod evolution_store;
#[cfg(feature = "postgres")]
pub mod postgres;
pub mod queue;
pub mod sqlite;
pub mod vector_store;

pub use agent_store::AgentStore;
pub use api_key_store::{validate_api_key, ApiKeyError, ApiKeyRecord, ApiKeyStore};
pub use audit_store::AuditStore;
pub use backend::{StorageBackend, StorageError, StorageExt};
pub use context_store::ContextStore;
#[cfg(feature = "postgres")]
pub use evolution_store::PostgresEvolutionStore;
pub use evolution_store::{EvolutionStore, EvolutionStoreError, SqliteEvolutionStore};
#[cfg(feature = "postgres")]
pub use postgres::PostgresBackend;
#[cfg(feature = "postgres")]
pub use queue::PostgresQueueBackend;
#[cfg(feature = "postgres")]
pub use vector_store::PgVectorStore;
pub use vector_store::{
    MemoryVectorStore, SqliteVectorStore, VectorEmbedding, VectorError, VectorStoreBackend,
};
