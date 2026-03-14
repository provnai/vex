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
//! // Returns: ["Over-generalization: Verify if universal claims 'always'/'never' hold true for all edge cases.", "Statistics: Determine if percentages are sourced or if they are illustrative placeholders."]
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
//! assert!(!consensus.reached); // 2/3 is not > 2/3
//! ```

pub mod consensus;
pub mod debate;
pub mod reflection;
pub mod shadow;

pub use consensus::{Consensus, ConsensusProtocol, SuperMajorityConfig, Vote};
pub use debate::{Debate, DebateRound};
pub use reflection::{ReflectionAgent, ReflectionConfig, ReflectionResult};
pub use shadow::{ShadowAgent, ShadowConfig};
