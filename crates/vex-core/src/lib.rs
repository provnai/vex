//! # VEX Core
//!
//! Core types for the VEX protocol — adversarial, temporal, cryptographically-verified AI agents.
//!
//! ## Key Types
//!
//! - [`Agent`] — Fractal agent with evolutionary capabilities
//! - [`ContextPacket`] — Time-aware, hashable context unit
//! - [`MerkleTree`] — Cryptographic verification of context hierarchies
//! - [`Genome`] — Evolutionary traits that map to LLM parameters
//!
//! ## Quick Start
//!
//! ```rust
//! use vex_core::{Agent, AgentConfig};
//!
//! // Create an agent
//! let agent = Agent::new(AgentConfig {
//!     name: "Researcher".to_string(),
//!     role: "You are a helpful research assistant".to_string(),
//!     max_depth: 3,
//!     spawn_shadow: true,
//! });
//!
//! // Spawn a child agent
//! let child = agent.spawn_child(AgentConfig {
//!     name: "Specialist".to_string(),
//!     role: "You analyze data".to_string(),
//!     max_depth: 2,
//!     spawn_shadow: false,
//! });
//! ```
//!
//! ## Merkle Verification
//!
//! ```rust
//! use vex_core::{MerkleTree, Hash};
//!
//! // Build a Merkle tree from context hashes
//! let leaves = vec![
//!     ("ctx1".to_string(), Hash::digest(b"context 1")),
//!     ("ctx2".to_string(), Hash::digest(b"context 2")),
//! ];
//! let tree = MerkleTree::from_leaves(leaves);
//!
//! // Verify integrity
//! assert!(tree.root_hash().is_some());
//! ```

pub mod agent;
pub mod audit;
pub mod context;
pub mod evolution;
pub mod evolution_memory;
pub mod fitness;
pub mod genome_experiment;
pub mod merkle;
pub mod rule;

pub use agent::{Agent, AgentConfig, AgentHandle, AgentId};
pub use audit::{ActorType, AuditEvent, AuditEventType, HashParams, Signature};
pub use context::{CompressionLevel, ContextPacket};
pub use evolution::{
    tournament_select, Fitness, GeneticOperator, Genome, LlmParams, StandardOperator,
};
pub use evolution_memory::{EvolutionMemory, TraitAdjustment};
pub use fitness::{EvaluationContext, FitnessEvaluator, FitnessReport, HeuristicEvaluator};
pub use genome_experiment::GenomeExperiment;
pub use merkle::{Hash, MerkleNode, MerkleProof, MerkleTree, ProofDirection, ProofStep};
pub use rule::OptimizationRule;
