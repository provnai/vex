//! Agent storage

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::backend::{StorageBackend, StorageError, StorageExt};
use vex_core::{Agent, AgentConfig};

/// Serializable agent state
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentState {
    pub id: Uuid,
    pub parent_id: Option<Uuid>,
    pub config: AgentConfig,
    pub generation: u32,
    pub fitness: f64,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

impl From<&Agent> for AgentState {
    fn from(agent: &Agent) -> Self {
        Self {
            id: agent.id,
            parent_id: agent.parent_id,
            config: agent.config.clone(),
            generation: agent.generation,
            fitness: agent.fitness,
            created_at: chrono::Utc::now(),
            updated_at: chrono::Utc::now(),
        }
    }
}

impl AgentState {
    /// Convert back to an Agent
    pub fn to_agent(&self) -> Agent {
        let mut agent = Agent::new(self.config.clone());
        agent.id = self.id;
        agent.parent_id = self.parent_id;
        agent.generation = self.generation;
        agent.fitness = self.fitness;
        agent
    }
}

/// Agent store for persistence
#[derive(Debug)]
pub struct AgentStore<B: StorageBackend + ?Sized> {
    backend: Arc<B>,
    prefix: String,
}

impl<B: StorageBackend + ?Sized> AgentStore<B> {
    /// Create a new agent store
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            prefix: "agent:".to_string(),
        }
    }

    /// Create with custom prefix
    pub fn with_prefix(backend: Arc<B>, prefix: &str) -> Self {
        Self {
            backend,
            prefix: prefix.to_string(),
        }
    }

    fn key(&self, tenant_id: &str, id: Uuid) -> String {
        format!("{}tenant:{}:{}", self.prefix, tenant_id, id)
    }

    /// Save an agent
    pub async fn save(&self, tenant_id: &str, agent: &Agent) -> Result<(), StorageError> {
        let state = AgentState::from(agent);
        self.backend
            .set(&self.key(tenant_id, agent.id), &state)
            .await
    }

    /// Load an agent by ID
    pub async fn load(&self, tenant_id: &str, id: Uuid) -> Result<Option<Agent>, StorageError> {
        let state: Option<AgentState> = self.backend.get(&self.key(tenant_id, id)).await?;
        Ok(state.map(|s| s.to_agent()))
    }

    /// Delete an agent
    pub async fn delete(&self, tenant_id: &str, id: Uuid) -> Result<bool, StorageError> {
        self.backend.delete(&self.key(tenant_id, id)).await
    }

    /// Check if agent exists
    pub async fn exists(&self, tenant_id: &str, id: Uuid) -> Result<bool, StorageError> {
        self.backend.exists(&self.key(tenant_id, id)).await
    }

    /// List all agent IDs for a tenant
    pub async fn list(&self, tenant_id: &str) -> Result<Vec<Uuid>, StorageError> {
        let tenant_prefix = format!("{}tenant:{}:", self.prefix, tenant_id);
        let keys = self.backend.list_keys(&tenant_prefix).await?;
        let ids: Vec<Uuid> = keys
            .iter()
            .filter_map(|k| {
                k.strip_prefix(&tenant_prefix)
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .collect();
        Ok(ids)
    }

    /// Load all agents for a tenant
    pub async fn load_all(&self, tenant_id: &str) -> Result<Vec<Agent>, StorageError> {
        let ids = self.list(tenant_id).await?;
        let mut agents = Vec::new();
        for id in ids {
            if let Some(agent) = self.load(tenant_id, id).await? {
                agents.push(agent);
            }
        }
        Ok(agents)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[tokio::test]
    async fn test_agent_store() {
        let backend = Arc::new(MemoryBackend::new());
        let store = AgentStore::new(backend);
        let tenant_id = "test-tenant";

        let agent = Agent::new(AgentConfig {
            name: "TestAgent".to_string(),
            role: "Tester".to_string(),
            max_depth: 2,
            spawn_shadow: true,
        });
        let id = agent.id;

        // Save
        store.save(tenant_id, &agent).await.unwrap();

        // Exists
        assert!(store.exists(tenant_id, id).await.unwrap());

        // Load
        let loaded = store.load(tenant_id, id).await.unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.config.name, "TestAgent");

        // List
        let ids = store.list(tenant_id).await.unwrap();
        assert_eq!(ids.len(), 1);

        // Delete
        assert!(store.delete(tenant_id, id).await.unwrap());
        assert!(!store.exists(tenant_id, id).await.unwrap());
    }
}
