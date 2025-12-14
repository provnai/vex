//! Agent executor - runs individual agents with LLM backend

use async_trait::async_trait;
use std::sync::Arc;
use uuid::Uuid;

use vex_core::{Agent, ContextPacket};
use vex_adversarial::{ShadowAgent, ShadowConfig, Debate, DebateRound, Consensus, ConsensusProtocol, Vote};

/// Configuration for agent execution
#[derive(Debug, Clone)]
pub struct ExecutorConfig {
    /// Maximum debate rounds
    pub max_debate_rounds: u32,
    /// Consensus protocol to use
    pub consensus_protocol: ConsensusProtocol,
    /// Whether to spawn shadow agents
    pub enable_adversarial: bool,
}

impl Default for ExecutorConfig {
    fn default() -> Self {
        Self {
            max_debate_rounds: 3,
            consensus_protocol: ConsensusProtocol::Majority,
            enable_adversarial: true,
        }
    }
}

/// Result of agent execution
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// The agent that produced this result
    pub agent_id: Uuid,
    /// The final response
    pub response: String,
    /// Whether it was verified by adversarial debate
    pub verified: bool,
    /// Confidence score (0.0 - 1.0)
    pub confidence: f64,
    /// Context packet with merkle hash
    pub context: ContextPacket,
    /// Debate details (if adversarial was enabled)
    pub debate: Option<Debate>,
}

/// Trait for LLM provider (re-exported for convenience)
#[async_trait]
pub trait LlmBackend: Send + Sync {
    async fn complete(&self, system: &str, prompt: &str) -> Result<String, String>;
}

/// Agent executor - runs agents with LLM backends
pub struct AgentExecutor<L: LlmBackend> {
    /// Configuration
    pub config: ExecutorConfig,
    /// LLM backend
    llm: Arc<L>,
}

impl<L: LlmBackend> AgentExecutor<L> {
    /// Create a new executor
    pub fn new(llm: Arc<L>, config: ExecutorConfig) -> Self {
        Self { config, llm }
    }

    /// Execute an agent with a prompt and return the result
    pub async fn execute(&self, agent: &mut Agent, prompt: &str) -> Result<ExecutionResult, String> {
        // Step 1: Get initial response from Blue agent
        let blue_response = self.llm
            .complete(&agent.config.role, prompt)
            .await?;

        // Step 2: If adversarial is enabled, run debate
        let (final_response, verified, confidence, debate) = if self.config.enable_adversarial {
            self.run_adversarial_verification(agent, prompt, &blue_response).await?
        } else {
            (blue_response, false, 0.5, None)
        };

        // Step 3: Create context packet with hash
        let mut context = ContextPacket::new(&final_response);
        context.source_agent = Some(agent.id);
        context.importance = confidence;

        // Step 4: Update agent's context
        agent.context = context.clone();
        agent.fitness = confidence;

        Ok(ExecutionResult {
            agent_id: agent.id,
            response: final_response,
            verified,
            confidence,
            context,
            debate,
        })
    }

    /// Run adversarial verification with Red agent
    async fn run_adversarial_verification(
        &self,
        blue_agent: &Agent,
        _original_prompt: &str,
        blue_response: &str,
    ) -> Result<(String, bool, f64, Option<Debate>), String> {
        // Create shadow agent
        let shadow = ShadowAgent::new(blue_agent, ShadowConfig::default());

        // Create debate
        let mut debate = Debate::new(blue_agent.id, shadow.agent.id, blue_response);

        // Run debate rounds
        for round_num in 1..=self.config.max_debate_rounds {
            // Red agent challenges
            let challenge_prompt = shadow.challenge_prompt(blue_response);
            let red_challenge = self.llm
                .complete(&shadow.agent.config.role, &challenge_prompt)
                .await?;

            // Blue agent rebuts (if there's something to rebut)
            let rebuttal = if red_challenge.to_lowercase().contains("disagree") 
                || red_challenge.to_lowercase().contains("issue")
                || red_challenge.to_lowercase().contains("concern")
            {
                let rebuttal_prompt = format!(
                    "Your previous response was challenged:\n\n\
                     Original: \"{}\"\n\n\
                     Challenge: \"{}\"\n\n\
                     Please address these concerns or revise your response.",
                    blue_response, red_challenge
                );
                Some(self.llm.complete(&blue_agent.config.role, &rebuttal_prompt).await?)
            } else {
                None
            };

            debate.add_round(DebateRound {
                round: round_num,
                blue_claim: blue_response.to_string(),
                red_challenge,
                blue_rebuttal: rebuttal,
            });

            // Check if we've reached consensus (Red agreed)
            if debate.rounds.last()
                .map(|r| r.red_challenge.to_lowercase().contains("agree"))
                .unwrap_or(false)
            {
                break;
            }
        }

        // Evaluate consensus
        let mut consensus = Consensus::new(self.config.consensus_protocol);

        // Blue's confidence depends on whether it successfully rebutted Red's challenges
        // If Blue had to make a rebuttal, confidence is reduced
        // If Red found issues Blue couldn't address, confidence is low
        let blue_confidence = if let Some(last_round) = debate.rounds.last() {
            let red_found_issues = last_round.red_challenge.to_lowercase().contains("issue")
                || last_round.red_challenge.to_lowercase().contains("concern")
                || last_round.red_challenge.to_lowercase().contains("disagree")
                || last_round.red_challenge.to_lowercase().contains("flaw");
            
            if red_found_issues {
                // Red found issues - Blue's confidence depends on rebuttal quality
                if last_round.blue_rebuttal.is_some() {
                    0.6 // Reduced confidence - had to defend
                } else {
                    0.3 // Very low - couldn't defend
                }
            } else {
                0.85 // Red agreed - high confidence
            }
        } else {
            0.5 // No debate rounds - neutral
        };

        consensus.add_vote(Vote {
            agent_id: blue_agent.id,
            agrees: true,
            confidence: blue_confidence,
            reasoning: Some(format!("Blue confidence: {:.0}%", blue_confidence * 100.0)),
        });

        // Red votes based on final challenge
        let red_agrees = debate.rounds.last()
            .map(|r| !r.red_challenge.to_lowercase().contains("disagree"))
            .unwrap_or(true);

        consensus.add_vote(Vote {
            agent_id: shadow.agent.id,
            agrees: red_agrees,
            confidence: 0.7,
            reasoning: debate.rounds.last().map(|r| r.red_challenge.clone()),
        });

        consensus.evaluate();

        // Determine final response
        let final_response = if consensus.reached && consensus.decision == Some(true) {
            blue_response.to_string()
        } else if let Some(last_round) = debate.rounds.last() {
            // Use rebuttal if available, otherwise original
            last_round.blue_rebuttal.clone().unwrap_or_else(|| blue_response.to_string())
        } else {
            blue_response.to_string()
        };

        let verified = consensus.reached;
        let confidence = consensus.confidence;

        debate.conclude(consensus.decision.unwrap_or(true), confidence);

        Ok((final_response, verified, confidence, Some(debate)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::AgentConfig;

    struct MockLlm;

    #[async_trait]
    impl LlmBackend for MockLlm {
        async fn complete(&self, _system: &str, prompt: &str) -> Result<String, String> {
            if prompt.contains("challenge") {
                Ok("I agree with this assessment. The logic is sound.".to_string())
            } else {
                Ok("This is a test response.".to_string())
            }
        }
    }

    #[tokio::test]
    async fn test_executor() {
        let llm = Arc::new(MockLlm);
        let executor = AgentExecutor::new(llm, ExecutorConfig::default());
        let mut agent = Agent::new(AgentConfig::default());

        let result = executor.execute(&mut agent, "Test prompt").await.unwrap();
        assert!(!result.response.is_empty());
        assert!(result.verified);
    }
}
