//! # VEX Temporal
//!
//! Time-aware memory compression for hierarchical agents.
//!
//! ## Features
//!
//! - Multi-scale time horizons (5 min → permanent)
//! - Automatic context decay with configurable strategies
//! - Episodic memory management with importance scoring
//!
//! ## Quick Start
//!
//! ```rust
//! use vex_temporal::{EpisodicMemory, Episode, HorizonConfig};
//!
//! // Create memory with custom config
//! let mut memory = EpisodicMemory::new(HorizonConfig {
//!     max_entries: 100,
//!     ..Default::default()
//! });
//!
//! // Remember events with importance scores
//! memory.remember("User asked about quantum computing", 0.8);
//! memory.remember("Provided detailed explanation", 0.9);
//!
//! // Add pinned memories (never evicted)
//! memory.add(Episode::pinned("System configuration"));
//!
//! // Get summary
//! println!("{}", memory.summarize());
//! ```
//!
//! ## Time Horizons
//!
//! | Horizon | Duration | Max Entries |
//! |---------|----------|-------------|
//! | Immediate | 5 min | 10 |
//! | ShortTerm | 1 hour | 25 |
//! | MediumTerm | 24 hours | 50 |
//! | LongTerm | 1 week | 100 |
//! | Permanent | ∞ | 500 |

pub mod compression;
pub mod horizon;
pub mod memory;

pub use compression::{DecayStrategy, TemporalCompressor};
pub use horizon::{HorizonConfig, TimeHorizon};
pub use memory::{Episode, EpisodicMemory};
