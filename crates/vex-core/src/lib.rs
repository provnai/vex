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
pub mod context;
pub mod evolution;
pub mod merkle;

pub use agent::{Agent, AgentConfig, AgentHandle, AgentId};
pub use context::{ContextPacket, CompressionLevel};
pub use evolution::{Fitness, GeneticOperator, Genome, LlmParams, StandardOperator, tournament_select};
pub use merkle::{Hash, MerkleNode, MerkleTree};

