//! # VEX Adversarial
//!
//! Red/Blue agent pairing for adversarial verification.
//!
//! Every "Blue" agent can have a "Red" shadow that challenges its conclusions,
//! reducing hallucinations and improving reliability.
//!
//! ## Key Types
//!
//! - [`ShadowAgent`] — Red agent that challenges Blue's outputs
//! - [`Debate`] — Multi-round debate between agents
//! - [`Consensus`] — Voting protocols for agreement
//!
//! ## Quick Start
//!
//! ```rust
//! use vex_adversarial::{ShadowAgent, ShadowConfig, Consensus, ConsensusProtocol, Vote};
//! use vex_core::{Agent, AgentConfig};
//!
//! // Create a Blue agent
//! let blue = Agent::new(AgentConfig::default());
//!
//! // Create its Red shadow
//! let shadow = ShadowAgent::new(&blue, ShadowConfig::default());
//!
//! // Detect issues in a claim
//! let issues = shadow.detect_issues("This always works 100% of the time.");
//! // Returns: ["Universal claim detected - verify no exceptions exist"]
//! ```
//!
//! ## Consensus Voting
//!
//! ```rust
//! use vex_adversarial::{Consensus, ConsensusProtocol, Vote};
//!
//! let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);
//! consensus.add_vote(Vote::new("agent1", true, 0.9));
//! consensus.add_vote(Vote::new("agent2", true, 0.8));
//! consensus.add_vote(Vote::new("agent3", false, 0.7));
//! consensus.evaluate();
//!
//! assert!(consensus.reached);
//! ```

pub mod consensus;
pub mod debate;
pub mod shadow;

pub use consensus::{Consensus, ConsensusProtocol, Vote};
pub use debate::{Debate, DebateRound};
pub use shadow::{ShadowAgent, ShadowConfig};
