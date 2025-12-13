//! # VEX Core
//!
//! Core types for the VEX protocol:
//! - [`Agent`] — Fractal agent with evolutionary capabilities
//! - [`ContextPacket`] — Time-aware, hashable context unit
//! - [`MerkleTree`] — Cryptographic verification of context hierarchies
//! - Evolution operators (crossover, mutation)

pub mod agent;
pub mod context;
pub mod evolution;
pub mod merkle;

pub use agent::{Agent, AgentConfig, AgentHandle, AgentId};
pub use context::{ContextPacket, CompressionLevel};
pub use evolution::{Fitness, GeneticOperator, Genome, LlmParams, StandardOperator, tournament_select};
pub use merkle::{Hash, MerkleNode, MerkleTree};
