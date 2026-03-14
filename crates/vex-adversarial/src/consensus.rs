//! Consensus protocols for multi-agent agreement

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A vote from an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Vote {
    /// Agent that cast this vote
    pub agent_id: Uuid,
    /// Whether they agree (true) or disagree (false)
    pub agrees: bool,
    /// Confidence in the vote (0.0 - 1.0)
    pub confidence: f64,
    /// Optional reasoning
    pub reasoning: Option<String>,
}

impl Vote {
    /// Create a new vote
    /// Uses SHA-256 hash of agent_id for UUID generation (collision resistant)
    pub fn new(agent_id: &str, agrees: bool, confidence: f64) -> Self {
        use sha2::{Digest, Sha256};

        // Hash the agent_id to get deterministic but collision-resistant bytes
        let mut hasher = Sha256::new();
        hasher.update(agent_id.as_bytes());
        let hash = hasher.finalize();

        // Take first 16 bytes of hash for UUID
        let mut bytes = [0u8; 16];
        bytes.copy_from_slice(&hash[..16]);

        Self {
            agent_id: Uuid::from_bytes(bytes),
            agrees,
            confidence,
            reasoning: None,
        }
    }
}

/// Type of consensus protocol
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ConsensusProtocol {
    /// Simple majority (> 50%)
    Majority,
    /// Super majority (> 66%)
    SuperMajority,
    /// Unanimous agreement
    Unanimous,
    /// Weighted by confidence scores
    WeightedConfidence,
}

/// Configuration for super-majority thresholds
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct SuperMajorityConfig {
    /// Ratio of agree votes required for consensus (default: 2/3)
    pub agree_threshold: f64,
    /// Ratio below which disagree consensus is reached (default: 1/3)
    pub disagree_threshold: f64,
}

impl Default for SuperMajorityConfig {
    fn default() -> Self {
        Self {
            agree_threshold: 2.0 / 3.0,
            disagree_threshold: 1.0 / 3.0,
        }
    }
}

/// Result of a consensus vote
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Consensus {
    /// The protocol used
    pub protocol: ConsensusProtocol,
    /// All votes cast
    pub votes: Vec<Vote>,
    /// Whether consensus was reached
    pub reached: bool,
    /// The consensus decision (if reached)
    pub decision: Option<bool>,
    /// Overall confidence
    pub confidence: f64,
    /// Super-majority thresholds (configurable)
    #[serde(default)]
    pub supermajority_config: SuperMajorityConfig,
}

impl Consensus {
    /// Create a new consensus with the given protocol
    pub fn new(protocol: ConsensusProtocol) -> Self {
        Self {
            protocol,
            votes: Vec::new(),
            reached: false,
            decision: None,
            confidence: 0.0,
            supermajority_config: SuperMajorityConfig::default(),
        }
    }

    /// Create with custom super-majority thresholds
    pub fn with_supermajority_config(mut self, config: SuperMajorityConfig) -> Self {
        self.supermajority_config = config;
        self
    }

    /// Add a vote
    pub fn add_vote(&mut self, vote: Vote) {
        self.votes.push(vote);
    }

    /// Evaluate the votes and determine consensus
    pub fn evaluate(&mut self) {
        if self.votes.is_empty() {
            return;
        }

        let total = self.votes.len() as f64;
        let agrees: f64 = self.votes.iter().filter(|v| v.agrees).count() as f64;
        if total == 0.0 {
            self.reached = false;
            self.decision = None;
            return;
        }

        let agree_ratio = agrees / total;

        let (reached, decision) = match self.protocol {
            ConsensusProtocol::Majority => (agree_ratio != 0.5, Some(agree_ratio > 0.5)),
            ConsensusProtocol::SuperMajority => {
                let cfg = &self.supermajority_config;
                if agree_ratio > cfg.agree_threshold {
                    (true, Some(true))
                } else if agree_ratio < cfg.disagree_threshold {
                    (true, Some(false))
                } else {
                    (false, None)
                }
            }
            ConsensusProtocol::Unanimous => {
                if agree_ratio == 1.0 {
                    (true, Some(true))
                } else if agree_ratio == 0.0 {
                    (true, Some(false))
                } else {
                    (false, None)
                }
            }
            ConsensusProtocol::WeightedConfidence => {
                let weighted_agree: f64 = self
                    .votes
                    .iter()
                    .filter(|v| v.agrees)
                    .map(|v| v.confidence)
                    .sum();
                let weighted_disagree: f64 = self
                    .votes
                    .iter()
                    .filter(|v| !v.agrees)
                    .map(|v| v.confidence)
                    .sum();
                let total_confidence = weighted_agree + weighted_disagree;

                if total_confidence > 0.0 {
                    let weighted_ratio = weighted_agree / total_confidence;
                    (true, Some(weighted_ratio > 0.5))
                } else {
                    (false, None)
                }
            }
        };

        self.reached = reached;
        self.decision = decision;
        if total == 0.0 {
            self.confidence = 0.0;
        } else {
            self.confidence = self.votes.iter().map(|v| v.confidence).sum::<f64>() / total;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_majority_consensus() {
        let mut consensus = Consensus::new(ConsensusProtocol::Majority);

        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: true,
            confidence: 0.9,
            reasoning: None,
        });
        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: true,
            confidence: 0.8,
            reasoning: None,
        });
        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: false,
            confidence: 0.7,
            reasoning: None,
        });

        consensus.evaluate();

        assert!(consensus.reached);
        assert_eq!(consensus.decision, Some(true));
    }

    #[test]
    fn test_empty_votes() {
        let mut consensus = Consensus::new(ConsensusProtocol::Majority);
        consensus.evaluate();
        assert!(!consensus.reached);
        assert_eq!(consensus.decision, None);
    }

    #[test]
    fn test_single_vote_majority() {
        let mut consensus = Consensus::new(ConsensusProtocol::Majority);
        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: true,
            confidence: 0.9,
            reasoning: None,
        });
        consensus.evaluate();
        assert!(consensus.reached);
        assert_eq!(consensus.decision, Some(true));
    }

    #[test]
    fn test_supermajority_boundary() {
        // Exactly at 2/3 should NOT reach consensus (requires > 2/3)
        let mut consensus = Consensus::new(ConsensusProtocol::SuperMajority);
        // 2 agree, 1 disagree = 66.67% which is > default 2/3
        for _ in 0..2 {
            consensus.add_vote(Vote {
                agent_id: Uuid::new_v4(),
                agrees: true,
                confidence: 0.8,
                reasoning: None,
            });
        }
        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: false,
            confidence: 0.8,
            reasoning: None,
        });
        consensus.evaluate();
        assert!(!consensus.reached);
        assert_eq!(consensus.decision, None);
    }

    #[test]
    fn test_deterministic_results() {
        // Same inputs should always produce same outputs
        for _ in 0..10 {
            let mut c = Consensus::new(ConsensusProtocol::WeightedConfidence);
            let id1 = Uuid::from_bytes([1; 16]);
            let id2 = Uuid::from_bytes([2; 16]);
            c.add_vote(Vote { agent_id: id1, agrees: true, confidence: 0.9, reasoning: None });
            c.add_vote(Vote { agent_id: id2, agrees: false, confidence: 0.3, reasoning: None });
            c.evaluate();
            assert!(c.reached);
            assert_eq!(c.decision, Some(true));
        }
    }

    #[test]
    fn test_unanimous_fails() {
        let mut consensus = Consensus::new(ConsensusProtocol::Unanimous);

        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: true,
            confidence: 0.9,
            reasoning: None,
        });
        consensus.add_vote(Vote {
            agent_id: Uuid::new_v4(),
            agrees: false,
            confidence: 0.8,
            reasoning: None,
        });

        consensus.evaluate();

        assert!(!consensus.reached);
        assert_eq!(consensus.decision, None);
    }
}
