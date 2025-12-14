//! Agent storage

use std::sync::Arc;
use uuid::Uuid;
use serde::{Deserialize, Serialize};

use vex_core::{Agent, AgentConfig};
use crate::backend::{StorageBackend, StorageExt, StorageError};

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

    fn key(&self, id: Uuid) -> String {
        format!("{}{}", self.prefix, id)
    }

    /// Save an agent
    pub async fn save(&self, agent: &Agent) -> Result<(), StorageError> {
        let state = AgentState::from(agent);
        self.backend.set(&self.key(agent.id), &state).await
    }

    /// Load an agent by ID
    pub async fn load(&self, id: Uuid) -> Result<Option<Agent>, StorageError> {
        let state: Option<AgentState> = self.backend.get(&self.key(id)).await?;
        Ok(state.map(|s| s.to_agent()))
    }

    /// Delete an agent
    pub async fn delete(&self, id: Uuid) -> Result<bool, StorageError> {
        self.backend.delete(&self.key(id)).await
    }

    /// Check if agent exists
    pub async fn exists(&self, id: Uuid) -> Result<bool, StorageError> {
        self.backend.exists(&self.key(id)).await
    }

    /// List all agent IDs
    pub async fn list(&self) -> Result<Vec<Uuid>, StorageError> {
        let keys = self.backend.list_keys(&self.prefix).await?;
        let ids: Vec<Uuid> = keys
            .iter()
            .filter_map(|k| {
                k.strip_prefix(&self.prefix)
                    .and_then(|s| Uuid::parse_str(s).ok())
            })
            .collect();
        Ok(ids)
    }

    /// Load all agents
    pub async fn load_all(&self) -> Result<Vec<Agent>, StorageError> {
        let ids = self.list().await?;
        let mut agents = Vec::new();
        for id in ids {
            if let Some(agent) = self.load(id).await? {
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

        let agent = Agent::new(AgentConfig {
            name: "TestAgent".to_string(),
            role: "Tester".to_string(),
            max_depth: 2,
            spawn_shadow: true,
        });
        let id = agent.id;

        // Save
        store.save(&agent).await.unwrap();

        // Exists
        assert!(store.exists(id).await.unwrap());

        // Load
        let loaded = store.load(id).await.unwrap().unwrap();
        assert_eq!(loaded.id, id);
        assert_eq!(loaded.config.name, "TestAgent");

        // List
        let ids = store.list().await.unwrap();
        assert_eq!(ids.len(), 1);

        // Delete
        assert!(store.delete(id).await.unwrap());
        assert!(!store.exists(id).await.unwrap());
    }
}
