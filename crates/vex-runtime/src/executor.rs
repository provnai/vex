//! Agent executor - runs individual agents with LLM backend

use std::sync::Arc;
use uuid::Uuid;

use serde::Deserialize;
use vex_adversarial::{
    Consensus, ConsensusProtocol, Debate, DebateRound, ShadowAgent, ShadowConfig, Vote,
};
use vex_core::{Agent, ContextPacket, Hash};

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    is_challenge: bool,
    confidence: f64,
    reasoning: String,
    suggested_revision: Option<String>,
}

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
    /// Logit-Merkle trace root (for provenance)
    pub trace_root: Option<Hash>,
    /// Debate details (if adversarial was enabled)
    pub debate: Option<Debate>,
}

use vex_llm::{LlmProvider, LlmRequest};

/// Agent executor - runs agents with LLM backends
pub struct AgentExecutor<L: LlmProvider> {
    /// Configuration
    pub config: ExecutorConfig,
    /// LLM backend
    llm: Arc<L>,
}

impl<L: LlmProvider> Clone for AgentExecutor<L> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            llm: self.llm.clone(),
        }
    }
}

impl<L: LlmProvider> AgentExecutor<L> {
    /// Create a new executor
    pub fn new(llm: Arc<L>, config: ExecutorConfig) -> Self {
        Self { config, llm }
    }

    /// Execute an agent with a prompt and return the result
    pub async fn execute(
        &self,
        agent: &mut Agent,
        prompt: &str,
    ) -> Result<ExecutionResult, String> {
        // Step 1: Format context and get initial response from Blue agent
        let full_prompt = if !agent.context.content.is_empty() {
            format!(
                "Previous Context (Time: {}):\n\"{}\"\n\nActive Prompt:\n\"{}\"",
                agent.context.created_at, agent.context.content, prompt
            )
        } else {
            prompt.to_string()
        };

        let blue_response = self
            .llm
            .complete(LlmRequest::with_role(&agent.config.role, &full_prompt))
            .await
            .map_err(|e| e.to_string())?
            .content;

        // Step 2: If adversarial is enabled, run debate
        let (final_response, verified, confidence, debate) = if self.config.enable_adversarial {
            self.run_adversarial_verification(agent, prompt, &blue_response)
                .await?
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
            trace_root: context.trace_root.clone(),
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

        // Initialize weighted consensus
        let mut consensus = Consensus::new(ConsensusProtocol::WeightedConfidence);

        // Run debate rounds
        for round_num in 1..=self.config.max_debate_rounds {
            // Red agent challenges
            let mut challenge_prompt = shadow.challenge_prompt(blue_response);
            challenge_prompt.push_str("\n\nIMPORTANT: Respond in valid JSON format: {\"is_challenge\": boolean, \"confidence\": float (0.0-1.0), \"reasoning\": \"string\", \"suggested_revision\": \"string\" | null}. If you agree with the statement, set is_challenge to false.");

            let red_output = self
                .llm
                .complete(LlmRequest::with_role(&shadow.agent.config.role, &challenge_prompt))
                .await
                .map_err(|e| e.to_string())?
                .content;

            // Try to parse JSON response
            let (is_challenge, red_confidence, red_reasoning, _suggested_revision) = 
                if let Some(start) = red_output.find('{') {
                    if let Some(end) = red_output.rfind('}') {
                        if let Ok(res) = serde_json::from_str::<ChallengeResponse>(&red_output[start..=end]) {
                            (res.is_challenge, res.confidence, res.reasoning, res.suggested_revision)
                        } else {
                            (red_output.to_lowercase().contains("disagree"), 0.5, red_output.clone(), None)
                        }
                    } else {
                        (false, 0.0, "Parsing failed".to_string(), None)
                    }
                } else {
                    (false, 0.0, "No JSON found".to_string(), None)
                };

            let rebuttal = if is_challenge {
                let rebuttal_prompt = format!(
                    "Your previous response was challenged by a Red agent:\n\n\
                     Original: \"{}\"\n\n\
                     Challenge: \"{}\"\n\n\
                     Please address these concerns or provide a revised response.",
                    blue_response, red_reasoning
                );
                Some(
                    self.llm
                        .complete(LlmRequest::with_role(&blue_agent.config.role, &rebuttal_prompt))
                        .await
                        .map_err(|e| e.to_string())?
                        .content,
                )
            } else {
                None
            };

            debate.add_round(DebateRound {
                round: round_num,
                blue_claim: blue_response.to_string(),
                red_challenge: red_reasoning.clone(),
                blue_rebuttal: rebuttal,
            });

            // Vote: Red votes based on whether it found a challenge
            consensus.add_vote(Vote {
                agent_id: shadow.agent.id,
                agrees: !is_challenge,
                confidence: red_confidence,
                reasoning: Some(red_reasoning),
            });

            if !is_challenge {
                break;
            }
        }

        // Blue votes with its evolved fitness as confidence (floor 0.5 for new agents)
        consensus.add_vote(Vote {
            agent_id: blue_agent.id,
            agrees: true,
            confidence: blue_agent.fitness.max(0.5),
            reasoning: Some(format!("Blue agent fitness: {:.0}%", blue_agent.fitness * 100.0)),
        });

        consensus.evaluate();

        // Determine final response
        let final_response = if consensus.reached && consensus.decision == Some(true) {
            blue_response.to_string()
        } else if let Some(last_round) = debate.rounds.last() {
            // Use rebuttal if available, otherwise original
            last_round
                .blue_rebuttal
                .clone()
                .unwrap_or_else(|| blue_response.to_string())
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

    #[tokio::test]
    async fn test_executor() {
        use vex_llm::MockProvider;
        let llm = Arc::new(MockProvider::smart());
        let executor = AgentExecutor::new(llm, ExecutorConfig::default());
        let mut agent = Agent::new(AgentConfig::default());

        let result = executor.execute(&mut agent, "Test prompt").await.unwrap();
        assert!(!result.response.is_empty());
        assert!(result.verified);
    }
}
