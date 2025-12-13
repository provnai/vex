//! # VEX Adversarial
//!
//! Red/Blue agent pairing for adversarial verification.
//!
//! Every "Blue" agent can have a "Red" shadow that challenges its conclusions,
//! reducing hallucinations and improving reliability.

pub mod consensus;
pub mod debate;
pub mod shadow;

pub use consensus::{Consensus, ConsensusProtocol, Vote};
pub use debate::{Debate, DebateRound};
pub use shadow::{ShadowAgent, ShadowConfig};
