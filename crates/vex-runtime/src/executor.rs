//! Agent executor - runs individual agents with LLM backend

use std::sync::Arc;
use uuid::Uuid;

use crate::gate::Gate;
use serde::Deserialize;
use vex_adversarial::{
    Consensus, ConsensusProtocol, Debate, DebateRound, ShadowAgent, ShadowConfig, Vote,
};
use vex_core::{Agent, ContextPacket, Hash};
use vex_hardware::api::AgentIdentity;
use vex_llm::Capability;
use vex_persist::{AuditStore, StorageBackend};

#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    is_challenge: bool,
    confidence: f64,
    reasoning: String,
    suggested_revision: Option<String>,
}

#[derive(Debug, Deserialize)]
struct VoteResponse {
    agrees: bool,
    reflection: String,
    confidence: f64,
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
    /// CHORA Evidence Capsule
    pub evidence: Option<vex_core::audit::EvidenceCapsule>,
}

use vex_llm::{LlmProvider, LlmRequest};

/// Agent executor - runs agents with LLM backends
pub struct AgentExecutor<L: LlmProvider + ?Sized> {
    /// Configuration
    pub config: ExecutorConfig,
    /// LLM backend
    llm: Arc<L>,
    /// Policy Gate
    gate: Arc<dyn Gate>,
    /// Audit Store (Phase 3)
    pub audit_store: Option<Arc<AuditStore<dyn StorageBackend>>>,
    /// Hardware Identity (Phase 3)
    pub identity: Option<Arc<AgentIdentity>>,
}

impl<L: LlmProvider + ?Sized> std::fmt::Debug for AgentExecutor<L> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AgentExecutor")
            .field("config", &self.config)
            .field("identity", &self.identity)
            .finish()
    }
}

impl<L: LlmProvider + ?Sized> Clone for AgentExecutor<L> {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            llm: self.llm.clone(),
            gate: self.gate.clone(),
            audit_store: self.audit_store.clone(),
            identity: self.identity.clone(),
        }
    }
}

impl<L: LlmProvider + ?Sized> AgentExecutor<L> {
    /// Create a new executor
    pub fn new(llm: Arc<L>, config: ExecutorConfig, gate: Arc<dyn Gate>) -> Self {
        Self {
            config,
            llm,
            gate,
            audit_store: None,
            identity: None,
        }
    }

    /// Attach a hardware-rooted identity and audit store (Phase 3)
    pub fn with_identity(
        mut self,
        identity: Arc<AgentIdentity>,
        audit_store: Arc<AuditStore<dyn StorageBackend>>,
    ) -> Self {
        self.identity = Some(identity);
        self.audit_store = Some(audit_store);
        self
    }

    /// Execute an agent with a prompt and return the result
    pub async fn execute(
        &self,
        tenant_id: &str, // Added tenant_id for audit logging
        agent: &mut Agent,
        prompt: &str,
        capabilities: Vec<Capability>,
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

        // Step 2.5: Policy Gate Verification (Mutation Risk Control)
        let capsule = self
            .gate
            .execute_gate(agent.id, prompt, &final_response, confidence, capabilities)
            .await;

        if capsule.outcome == "HALT" {
            return Err(format!("Gate Blocking: {}", capsule.reason_code));
        }

        // Step 3: Create context packet with hash
        let mut context = ContextPacket::new(&final_response);
        context.source_agent = Some(agent.id);
        context.importance = confidence;

        // Step 4: Update agent's context
        agent.context = context.clone();
        agent.fitness = confidence;

        let result = ExecutionResult {
            agent_id: agent.id,
            response: final_response,
            verified,
            confidence,
            trace_root: context.trace_root.clone(),
            context: context.clone(),
            debate,
            evidence: Some(capsule.clone()),
        };

        // Step 5: Automatic Hardware-Signed Audit Log (Phase 3)
        if let Some(store) = &self.audit_store {
            let _ = store
                .log(
                    tenant_id,
                    vex_core::audit::AuditEventType::AgentExecuted,
                    vex_core::audit::ActorType::Bot(agent.id),
                    Some(agent.id),
                    serde_json::json!({
                        "prompt": prompt,
                        "confidence": confidence,
                        "verified": verified,
                    }),
                    self.identity.as_ref().map(|id| id.as_ref()),
                    Some(capsule.witness_receipt.clone()),
                    capsule.vep_blob.clone(),
                )
                .await;
        }

        Ok(result)
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
                .complete(LlmRequest::with_role(
                    &shadow.agent.config.role,
                    &challenge_prompt,
                ))
                .await
                .map_err(|e| e.to_string())?
                .content;

            // Try to parse JSON response — fail closed on parse errors
            let (is_challenge, red_confidence, red_reasoning, _suggested_revision) =
                if let Ok(res) = serde_json::from_str::<ChallengeResponse>(&red_output) {
                    (
                        res.is_challenge,
                        res.confidence,
                        res.reasoning,
                        res.suggested_revision,
                    )
                } else if let Some(start) = red_output.find('{') {
                    if let Some(end) = red_output.rfind('}') {
                        if let Ok(res) =
                            serde_json::from_str::<ChallengeResponse>(&red_output[start..=end])
                        {
                            (
                                res.is_challenge,
                                res.confidence,
                                res.reasoning,
                                res.suggested_revision,
                            )
                        } else {
                            // Fail closed: treat unparseable response as a challenge
                            (true, 0.5, red_output.clone(), None)
                        }
                    } else {
                        // Fail closed
                        (true, 0.5, "Parsing failed".to_string(), None)
                    }
                } else {
                    // Fail closed
                    (true, 0.5, "No JSON found".to_string(), None)
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
                        .complete(LlmRequest::with_role(
                            &blue_agent.config.role,
                            &rebuttal_prompt,
                        ))
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

        // Blue agent reflects on the debate and decides its final vote (Fix for #3 bias)
        let mut reflection_prompt = format!(
            "You have just finished an adversarial debate about your original response.\n\n\
             Original Response: \"{}\"\n\n\
             Debate Rounds:\n",
            blue_response
        );

        for (i, round) in debate.rounds.iter().enumerate() {
            reflection_prompt.push_str(&format!(
                "Round {}: Red challenged: \"{}\" -> You rebutted: \"{}\"\n",
                i + 1,
                round.red_challenge,
                round.blue_rebuttal.as_deref().unwrap_or("N/A")
            ));
        }

        reflection_prompt.push_str("\nBased on this debate, do you still stand by your original response? \
                                    Respond in valid JSON: {\"agrees\": boolean, \"confidence\": float (0.0-1.0), \"reasoning\": \"string\"}.");

        let blue_vote_res = self
            .llm
            .complete(LlmRequest::with_role(
                &blue_agent.config.role,
                &reflection_prompt,
            ))
            .await;

        // Fail closed: on parse failure, blue does NOT agree (conservative)
        let (blue_agrees, blue_confidence, blue_reasoning) = if let Ok(resp) = blue_vote_res {
            if let Ok(vote) = serde_json::from_str::<VoteResponse>(&resp.content) {
                (vote.agrees, vote.confidence, vote.reflection)
            } else if let Some(start) = resp.content.find('{') {
                if let Some(end) = resp.content.rfind('}') {
                    if let Ok(vote) =
                        serde_json::from_str::<VoteResponse>(&resp.content[start..=end])
                    {
                        (vote.agrees, vote.confidence, vote.reflection)
                    } else {
                        (
                            false,
                            blue_agent.fitness,
                            "Failed to parse reflection JSON".to_string(),
                        )
                    }
                } else {
                    (
                        false,
                        blue_agent.fitness,
                        "No JSON in reflection".to_string(),
                    )
                }
            } else {
                (
                    false,
                    blue_agent.fitness,
                    "No reflection content".to_string(),
                )
            }
        } else {
            (
                false,
                blue_agent.fitness,
                "Reflection LLM call failed".to_string(),
            )
        };

        consensus.add_vote(Vote {
            agent_id: blue_agent.id,
            agrees: blue_agrees,
            confidence: blue_confidence,
            reasoning: Some(blue_reasoning),
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

        // Fail closed: if no consensus decision, reject the claim
        debate.conclude(consensus.decision.unwrap_or(false), confidence);

        Ok((final_response, verified, confidence, Some(debate)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use vex_core::AgentConfig;

    #[tokio::test]
    async fn test_executor() {
        use crate::gate::GenericGateMock;
        use vex_llm::MockProvider;
        let llm = Arc::new(MockProvider::smart());
        let gate = Arc::new(GenericGateMock);
        let config = ExecutorConfig {
            enable_adversarial: false,
            ..Default::default()
        };
        let executor = AgentExecutor::new(llm, config, gate);
        let mut agent = Agent::new(AgentConfig::default());

        let result = executor
            .execute("test-tenant", &mut agent, "Test prompt", vec![])
            .await
            .unwrap();
        assert!(!result.response.is_empty());
        // verified is false by design when enable_adversarial = false
        assert!(!result.verified);
    }
}
