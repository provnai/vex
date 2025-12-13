//! # VEX Temporal
//!
//! Time-aware memory compression for hierarchical agents.
//!
//! Features:
//! - Multi-scale time horizons
//! - Automatic context decay
//! - Episodic memory management

pub mod horizon;
pub mod compression;
pub mod memory;

pub use horizon::{TimeHorizon, HorizonConfig};
pub use compression::{TemporalCompressor, DecayStrategy};
pub use memory::{EpisodicMemory, Episode};
