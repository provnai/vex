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
        }
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
                if agree_ratio > 0.66 {
                    (true, Some(true))
                } else if agree_ratio < 0.34 {
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
