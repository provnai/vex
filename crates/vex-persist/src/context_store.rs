//! Context packet storage

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::backend::{StorageBackend, StorageError, StorageExt};
use vex_core::ContextPacket;

/// Serializable context state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextState {
    /// Unique ID for this context
    pub id: Uuid,
    /// The context packet
    pub packet: ContextPacket,
    /// Agent that created this context
    pub agent_id: Option<Uuid>,
    /// When it was stored
    pub stored_at: DateTime<Utc>,
}

/// Context store for persistence
#[derive(Debug)]
pub struct ContextStore<B: StorageBackend + ?Sized> {
    backend: Arc<B>,
    prefix: String,
}

impl<B: StorageBackend + ?Sized> ContextStore<B> {
    /// Create a new context store
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            prefix: "context:".to_string(),
        }
    }

    fn key(&self, id: Uuid) -> String {
        format!("{}{}", self.prefix, id)
    }

    fn agent_key(&self, agent_id: Uuid) -> String {
        format!("{}agent:{}", self.prefix, agent_id)
    }

    /// Save a context packet
    pub async fn save(&self, packet: &ContextPacket) -> Result<Uuid, StorageError> {
        let id = Uuid::new_v4();
        let state = ContextState {
            id,
            packet: packet.clone(),
            agent_id: packet.source_agent,
            stored_at: Utc::now(),
        };
        self.backend.set(&self.key(id), &state).await?;

        // Also index by agent if available
        if let Some(agent_id) = packet.source_agent {
            let mut agent_contexts: Vec<Uuid> = self
                .backend
                .get(&self.agent_key(agent_id))
                .await?
                .unwrap_or_default();
            agent_contexts.push(id);
            self.backend
                .set(&self.agent_key(agent_id), &agent_contexts)
                .await?;
        }

        Ok(id)
    }

    /// Load a context by ID
    pub async fn load(&self, id: Uuid) -> Result<Option<ContextPacket>, StorageError> {
        let state: Option<ContextState> = self.backend.get(&self.key(id)).await?;
        Ok(state.map(|s| s.packet))
    }

    /// Load all contexts for an agent
    pub async fn load_by_agent(&self, agent_id: Uuid) -> Result<Vec<ContextPacket>, StorageError> {
        let context_ids: Vec<Uuid> = self
            .backend
            .get(&self.agent_key(agent_id))
            .await?
            .unwrap_or_default();

        let mut contexts = Vec::new();
        for id in context_ids {
            if let Some(ctx) = self.load(id).await? {
                contexts.push(ctx);
            }
        }
        Ok(contexts)
    }

    /// Delete a context
    pub async fn delete(&self, id: Uuid) -> Result<bool, StorageError> {
        self.backend.delete(&self.key(id)).await
    }

    /// Get total count of stored contexts
    pub async fn count(&self) -> Result<usize, StorageError> {
        let keys = self.backend.list_keys(&self.prefix).await?;
        Ok(keys.iter().filter(|k| !k.contains(":agent:")).count())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[tokio::test]
    async fn test_context_store() {
        let backend = Arc::new(MemoryBackend::new());
        let store = ContextStore::new(backend);

        let mut packet = ContextPacket::new("Test content");
        let agent_id = Uuid::new_v4();
        packet.source_agent = Some(agent_id);

        // Save
        let id = store.save(&packet).await.unwrap();

        // Load
        let loaded = store.load(id).await.unwrap().unwrap();
        assert_eq!(loaded.content, "Test content");

        // Load by agent
        let agent_contexts = store.load_by_agent(agent_id).await.unwrap();
        assert_eq!(agent_contexts.len(), 1);

        // Count
        assert_eq!(store.count().await.unwrap(), 1);
    }
}
