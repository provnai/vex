//! Debate protocol between Blue and Red agents

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A single round in a debate
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DebateRound {
    /// Round number
    pub round: u32,
    /// Blue agent's claim/response
    pub blue_claim: String,
    /// Red agent's challenge
    pub red_challenge: String,
    /// Blue agent's rebuttal (if any)
    pub blue_rebuttal: Option<String>,
}

/// A complete debate between Blue and Red agents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Debate {
    /// Unique ID
    pub id: Uuid,
    /// Blue agent ID
    pub blue_agent_id: Uuid,
    /// Red agent ID
    pub red_agent_id: Uuid,
    /// The initial claim being debated
    pub initial_claim: String,
    /// Rounds of debate
    pub rounds: Vec<DebateRound>,
    /// Final verdict (true = claim upheld, false = claim rejected)
    pub verdict: Option<bool>,
    /// Confidence in the verdict (0.0 - 1.0)
    pub confidence: f64,
}

impl Debate {
    /// Create a new debate
    pub fn new(blue_id: Uuid, red_id: Uuid, claim: &str) -> Self {
        Self {
            id: Uuid::new_v4(),
            blue_agent_id: blue_id,
            red_agent_id: red_id,
            initial_claim: claim.to_string(),
            rounds: Vec::new(),
            verdict: None,
            confidence: 0.0,
        }
    }

    /// Add a round to the debate
    pub fn add_round(&mut self, round: DebateRound) {
        self.rounds.push(round);
    }

    /// Conclude the debate with a verdict
    pub fn conclude(&mut self, upheld: bool, confidence: f64) {
        self.verdict = Some(upheld);
        self.confidence = confidence.clamp(0.0, 1.0);
    }

    /// Check if debate is concluded
    pub fn is_concluded(&self) -> bool {
        self.verdict.is_some()
    }

    /// Get number of rounds
    pub fn round_count(&self) -> usize {
        self.rounds.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_debate_lifecycle() {
        let mut debate = Debate::new(Uuid::new_v4(), Uuid::new_v4(), "The sky is blue");

        assert!(!debate.is_concluded());

        debate.add_round(DebateRound {
            round: 1,
            blue_claim: "The sky is blue due to Rayleigh scattering".to_string(),
            red_challenge: "But the sky is red at sunset".to_string(),
            blue_rebuttal: Some("Rayleigh scattering still applies...".to_string()),
        });

        assert_eq!(debate.round_count(), 1);

        debate.conclude(true, 0.85);
        assert!(debate.is_concluded());
        assert_eq!(debate.verdict, Some(true));
    }
}
