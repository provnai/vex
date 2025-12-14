//! Agent types for VEX
//!
//! The core [`Agent`] struct represents a fractal agent in the hierarchy.

use crate::context::ContextPacket;
use crate::evolution::{Genome, LlmParams};
use crate::merkle::Hash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

/// Unique identifier for an agent
pub type AgentId = Uuid;

/// Handle to a running agent (thread-safe reference)
pub type AgentHandle = Arc<RwLock<Agent>>;

/// Configuration for creating a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Human-readable name for this agent
    pub name: String,
    /// Role description (used in LLM prompts)
    pub role: String,
    /// Maximum depth of child agents allowed
    pub max_depth: u8,
    /// Whether this agent should spawn a shadow (adversarial) agent
    pub spawn_shadow: bool,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            name: "Agent".to_string(),
            role: "General purpose assistant".to_string(),
            max_depth: 3,
            spawn_shadow: true,
        }
    }
}

/// A fractal agent in the VEX hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub id: AgentId,
    /// Configuration
    pub config: AgentConfig,
    /// Generation number (0 = root, increases with each fork)
    pub generation: u32,
    /// Current depth in the hierarchy
    pub depth: u8,
    /// The agent's current context/memory
    pub context: ContextPacket,
    /// Merkle root of all child context hashes
    pub merkle_root: Option<Hash>,
    /// When this agent was created
    pub created_at: DateTime<Utc>,
    /// IDs of child agents (not serialized, reconstructed at runtime)
    #[serde(skip)]
    pub children: Vec<AgentId>,
    /// ID of shadow (adversarial) agent if spawned
    #[serde(skip)]
    pub shadow_id: Option<AgentId>,
    /// ID of parent agent (None for root)
    pub parent_id: Option<AgentId>,
    /// Fitness score from last evaluation
    pub fitness: f64,
    /// Genome encoding agent traits (evolved over generations)
    pub genome: Genome,
}

impl Agent {
    /// Create a new root agent with the given configuration
    pub fn new(config: AgentConfig) -> Self {
        // Create genome before moving config into struct
        let genome = Genome::new(&config.role);

        Self {
            id: Uuid::new_v4(),
            config,
            generation: 0,
            depth: 0,
            context: ContextPacket::new(""),
            merkle_root: None,
            created_at: Utc::now(),
            children: Vec::new(),
            shadow_id: None,
            parent_id: None,
            fitness: 0.0,
            genome,
        }
    }

    /// Create a child agent from this agent (inherits parent's genome)
    pub fn spawn_child(&self, config: AgentConfig) -> Self {
        // Child inherits parent's genome (will be evolved by orchestrator if enabled)
        let mut child_genome = self.genome.clone();
        child_genome.prompt = config.role.clone();

        Self {
            id: Uuid::new_v4(),
            config,
            generation: self.generation + 1,
            depth: self.depth + 1,
            context: ContextPacket::new(""),
            merkle_root: None,
            created_at: Utc::now(),
            children: Vec::new(),
            shadow_id: None,
            parent_id: Some(self.id),
            fitness: 0.0,
            genome: child_genome,
        }
    }

    /// Check if this agent can spawn more children
    pub fn can_spawn(&self) -> bool {
        self.depth < self.config.max_depth
    }

    /// Check if this agent is a root agent
    pub fn is_root(&self) -> bool {
        self.parent_id.is_none()
    }

    /// Get LLM parameters derived from this agent's genome
    pub fn llm_params(&self) -> LlmParams {
        self.genome.to_llm_params()
    }

    /// Update genome from an evolved offspring
    pub fn apply_evolved_genome(&mut self, evolved: Genome) {
        self.genome = evolved;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent() {
        let agent = Agent::new(AgentConfig::default());
        assert!(agent.is_root());
        assert_eq!(agent.generation, 0);
        assert_eq!(agent.depth, 0);
    }

    #[test]
    fn test_spawn_child() {
        let parent = Agent::new(AgentConfig::default());
        let child = parent.spawn_child(AgentConfig::default());

        assert!(!child.is_root());
        assert_eq!(child.generation, 1);
        assert_eq!(child.depth, 1);
        assert_eq!(child.parent_id, Some(parent.id));
    }
}
