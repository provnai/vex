use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info, warn};
use uuid::Uuid;
use vex_adversarial::{
    Consensus, ConsensusProtocol, Debate, DebateRound, ShadowAgent, ShadowConfig, Vote,
};
use vex_core::{Agent, AgentConfig, ContextPacket};
use vex_llm::{LlmProvider, LlmRequest};
use vex_persist::{AgentStore, StorageBackend};
use vex_queue::job::BackoffStrategy;
use vex_queue::{Job, JobResult};

/// Payload for agent execution job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJobPayload {
    pub agent_id: String,
    pub prompt: String,
    pub context_id: Option<String>,
    #[serde(default)]
    pub enable_adversarial: bool,
    #[serde(default)]
    pub enable_self_correction: bool,
    #[serde(default = "default_max_rounds")]
    pub max_debate_rounds: u32,
    pub tenant_id: Option<String>,
}

fn default_max_rounds() -> u32 {
    3
}

/// Result of an agent execution job — now includes VEX verification fields
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJobResult {
    pub job_id: Uuid,
    pub agent_id: String,
    pub prompt: String,
    pub response: String,
    pub tokens_used: Option<u32>,
    pub completed_at: DateTime<Utc>,
    pub success: bool,
    pub error: Option<String>,
    // --- VEX Provenance Fields ---
    /// Whether the response was verified by adversarial debate
    pub verified: bool,
    /// Consensus confidence score (0.0–1.0)
    pub confidence: f64,
    /// SHA-256 hash of the response content (ContextPacket)
    pub context_hash: Option<String>,
    /// Number of debate rounds that occurred
    pub debate_rounds: u32,
    /// Merkle Tree root of all ContextPackets
    pub merkle_root: Option<String>,
    /// ID of the final ContextPacket generated from this job
    pub new_context_id: Option<String>,
    /// CHORA Evidence Capsule for this execution
    pub evidence: Option<vex_core::audit::EvidenceCapsule>,
}

/// Shared storage for job results
pub type JobResultStore = Arc<RwLock<HashMap<Uuid, AgentJobResult>>>;

/// Create a new job result store
pub fn new_result_store() -> JobResultStore {
    Arc::new(RwLock::new(HashMap::new()))
}

#[derive(Debug)]
pub struct AgentExecutionJob {
    pub job_id: Uuid,
    pub payload: AgentJobPayload,
    pub llm: Arc<dyn LlmProvider>,
    pub result_store: JobResultStore,
    pub db: Arc<dyn StorageBackend>,
    pub anchor: Option<Arc<dyn vex_anchor::AnchorBackend>>,
    pub evolution_store: Arc<dyn vex_persist::EvolutionStore>,
    pub gate: Arc<dyn vex_runtime::Gate>,
}

impl AgentExecutionJob {
    pub fn new(
        job_id: Uuid,
        payload: AgentJobPayload,
        llm: Arc<dyn LlmProvider>,
        result_store: JobResultStore,
        db: Arc<dyn StorageBackend>,
        anchor: Option<Arc<dyn vex_anchor::AnchorBackend>>,
        evolution_store: Arc<dyn vex_persist::EvolutionStore>,
        gate: Arc<dyn vex_runtime::Gate>,
    ) -> Self {
        Self {
            job_id,
            payload,
            llm,
            result_store,
            db,
            anchor,
            evolution_store,
            gate,
        }
    }
}

/// JSON shapes for structured adversarial responses
#[derive(Debug, Deserialize)]
struct ChallengeResponse {
    is_challenge: bool,
    confidence: f64,
    reasoning: String,
}

#[derive(Debug, Deserialize)]
struct VoteResponse {
    agrees: bool,
    reflection: String,
    confidence: f64,
}

#[async_trait]
impl Job for AgentExecutionJob {
    fn name(&self) -> &str {
        "agent_execution"
    }

    async fn execute(&mut self) -> JobResult {
        info!(
            job_id = %self.job_id,
            agent_id = %self.payload.agent_id,
            adversarial = self.payload.enable_adversarial,
            "Executing VEX agent job"
        );

        // --- Step 1: Load agent role from DB (use tenant_id for isolation) ---
        let agent_id_uuid = Uuid::parse_str(&self.payload.agent_id).unwrap_or_else(|_| Uuid::nil());
        let tenant_id = self.payload.tenant_id.as_deref().unwrap_or("default");

        let (agent_role, temperature, top_p, presence_penalty, frequency_penalty) = {
            let store = AgentStore::new(self.db.clone());
            match store.load(tenant_id, agent_id_uuid).await {
                Ok(Some(a)) => {
                    info!(job_id = %self.job_id, role = %a.config.role, "Loaded agent role from DB");
                    let params = a.genome.to_llm_params();
                    (
                        a.config.role.clone(),
                        params.temperature as f32,
                        Some(params.top_p as f32),
                        Some(params.presence_penalty as f32),
                        Some(params.frequency_penalty as f32),
                    )
                }
                Ok(None) => {
                    warn!(job_id = %self.job_id, "Agent not found in DB, using default role");
                    (
                        "You are a helpful and precise VEX agent.".to_string(),
                        0.7,
                        None,
                        None,
                        None,
                    )
                }
                Err(e) => {
                    warn!(job_id = %self.job_id, error = %e, "Failed to load agent, using default role");
                    (
                        "You are a helpful and precise VEX agent.".to_string(),
                        0.7,
                        None,
                        None,
                        None,
                    )
                }
            }
        };

        // --- Step 1.5: Fetch prior context if provided ---
        let memory_context = if let Some(ref ctx_id) = self.payload.context_id {
            if let Ok(ctx_uuid) = Uuid::parse_str(ctx_id) {
                let ctx_store = vex_persist::ContextStore::new(self.db.clone());
                ctx_store
                    .load(tenant_id, ctx_uuid)
                    .await
                    .ok()
                    .flatten()
                    .map(|pkt| pkt.content)
            } else {
                None
            }
        } else {
            None
        };

        // --- Step 2: Blue agent — initial LLM response ---
        let full_prompt = match memory_context {
            Some(mem) => format!(
                "Previous context:\n\"{}\"\n\nCurrent task:\n\"{}\"",
                mem, self.payload.prompt
            ),
            None => self.payload.prompt.clone(),
        };

        let mut blue_request = LlmRequest::with_role(&agent_role, &full_prompt);
        blue_request.temperature = temperature;
        blue_request.top_p = top_p;
        blue_request.presence_penalty = presence_penalty;
        blue_request.frequency_penalty = frequency_penalty;
        let blue_response = match self.llm.complete(blue_request).await {
            Ok(r) => r,
            Err(e) => {
                vex_llm::global_metrics().record_llm_call(0, true);
                error!(job_id = %self.job_id, error = %e, "Blue agent LLM call failed");
                return self.store_error(e.to_string()).await;
            }
        };

        let tokens_blue = blue_response.tokens_used.unwrap_or(0);
        vex_llm::global_metrics().record_llm_call(tokens_blue as u64, false);

        let blue_content = blue_response.content.clone();
        let mut total_tokens = tokens_blue;
        let mut context_hashes: Vec<vex_core::Hash> = Vec::new();

        // Track Blue's initial response
        let blue_initial_context = ContextPacket::new(&blue_content);
        context_hashes.push(blue_initial_context.hash);

        // --- Step 3: Adversarial debate (if enabled) ---
        let (final_response, verified, confidence, debate_rounds) = if self
            .payload
            .enable_adversarial
        {
            match self
                .run_adversarial_debate(&agent_role, &blue_content)
                .await
            {
                Ok((resp, ver, conf, rounds, extra_tokens, debate_hashes)) => {
                    total_tokens += extra_tokens;
                    vex_llm::global_metrics().record_llm_call(extra_tokens as u64, false);
                    context_hashes.extend(debate_hashes);
                    (resp, ver, conf, rounds)
                }
                Err(e) => {
                    warn!(job_id = %self.job_id, error = %e, "Adversarial debate failed, using raw Blue response");
                    (blue_content.clone(), false, 0.5, 0)
                }
            }
        } else {
            (blue_content.clone(), false, 0.5, 0)
        };

        // Tracking final response
        let final_context = ContextPacket::new(&final_response);
        context_hashes.push(final_context.hash.clone());

        // --- Step 3.5: CHORA Gate Decision Boundary ---
        let capsule = self
            .gate
            .execute_gate(
                agent_id_uuid,
                &self.payload.prompt,
                &final_response,
                confidence,
            )
            .await;

        if capsule.outcome == "HALT" {
            warn!(
                job_id = %self.job_id,
                reason = %capsule.reason_code,
                "CHORA Gate transition: HALT. Execution blocked."
            );
            return self
                .store_error(format!("CHORA Gate Blocking: {}", capsule.reason_code))
                .await;
        }

        info!(
            job_id = %self.job_id,
            capsule_id = %capsule.capsule_id,
            "CHORA Gate transition: ALLOW"
        );

        // --- Step 4: Create Merkle Root for the entire execution ---
        let leaves: Vec<(String, vex_core::Hash)> = context_hashes
            .into_iter()
            .enumerate()
            .map(|(i, h)| (format!("packet_{}", i), h))
            .collect();
        let tree = vex_core::MerkleTree::from_leaves(leaves);
        let merkle_root = tree
            .root_hash()
            .map(|h| h.0.iter().map(|b| format!("{:02x}", b)).collect::<String>());

        let context_hash = final_context
            .hash
            .0
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect::<String>();

        // --- Step 4.5: Anchor the Merkle Root if FileAnchor is provided ---
        if let (Some(anchor), Some(root_hash)) = (&self.anchor, tree.root_hash()) {
            let meta = vex_anchor::AnchorMetadata::new(&self.payload.agent_id, 1);
            match anchor.anchor(root_hash, meta).await {
                Ok(receipt) => {
                    info!(
                        anchor_id = %receipt.anchor_id,
                        backend = %receipt.backend,
                        "Execution Merkle root anchored"
                    );
                }
                Err(e) => {
                    warn!(job_id = %self.job_id, error = %e, "Merkle root anchoring failed (non-fatal)")
                }
            }
        }
        // --- Step 4.7 & 4.7.5: Record genome experiment and trigger reflection ---
        if let Ok(Some(agent)) = AgentStore::new(self.db.clone())
            .load(tenant_id, agent_id_uuid)
            .await
        {
            let experiment = vex_core::GenomeExperiment::new(
                &agent.genome,
                std::collections::HashMap::new(),
                confidence, // fitness = debate confidence
                &self.payload.prompt,
            );

            if let Err(e) = self
                .evolution_store
                .save_experiment(tenant_id, &experiment)
                .await
            {
                warn!(job_id = %self.job_id, error = %e, "Failed to save evolution experiment");
            }

            // --- Step 4.7.5: Trigger reflection every 5 executions (if enabled) ---
            if self.payload.enable_self_correction {
                let recent_count = self
                    .evolution_store
                    .count_experiments(tenant_id)
                    .await
                    .unwrap_or(0);
                if recent_count > 0 && recent_count % 5 == 0 {
                    let reflection_agent = vex_adversarial::ReflectionAgent::new(self.llm.clone());
                    let experiments = self
                        .evolution_store
                        .load_recent(tenant_id, 10)
                        .await
                        .unwrap_or_default();
                    let mut evo_memory = vex_core::EvolutionMemory::new();
                    for exp in experiments {
                        evo_memory.record(exp);
                    }

                    // Let the agent analyze the current run + memory and provide suggestions
                    let suggestions = reflection_agent
                        .reflect(
                            &agent,
                            &self.payload.prompt,
                            &final_response,
                            confidence,
                            &evo_memory,
                        )
                        .await;
                    if suggestions.has_adjustments() {
                        info!(
                            agent_id = %self.payload.agent_id,
                            adjustments = ?suggestions.adjustments,
                            "ReflectionAgent genome suggestions (every 5 tasks)"
                        );
                    }
                }
            }
        }

        // --- Step 4.8: Save context packet for subsequent memory ---
        let ctx_store = vex_persist::ContextStore::new(self.db.clone());
        let new_packet = ContextPacket::new(&final_response);
        let new_context_id = new_packet.id;
        if let Err(e) = ctx_store.save(tenant_id, &new_packet).await {
            warn!(job_id = %self.job_id, error = %e, "Failed to save context packet");
        }

        info!(
            job_id = %self.job_id,
            verified = verified,
            confidence = confidence,
            debate_rounds = debate_rounds,
            context_hash = %context_hash,
            tokens = total_tokens,
            "Agent job completed"
        );

        // --- Step 5: Store enriched result ---
        let result = AgentJobResult {
            job_id: self.job_id,
            agent_id: self.payload.agent_id.clone(),
            prompt: self.payload.prompt.clone(),
            response: final_response,
            tokens_used: Some(total_tokens),
            completed_at: Utc::now(),
            success: true,
            error: None,
            verified,
            confidence,
            context_hash: Some(context_hash),
            debate_rounds,
            merkle_root: merkle_root.clone(),
            new_context_id: Some(new_context_id.to_string()),
            evidence: Some(capsule),
        };

        self.result_store
            .write()
            .await
            .insert(self.job_id, result.clone());

        JobResult::Success(Some(serde_json::to_value(&result).unwrap()))
    }

    fn max_retries(&self) -> u32 {
        5
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential {
            initial_secs: 2,
            multiplier: 2.0,
        }
    }
}

impl AgentExecutionJob {
    /// Run the Blue/Red adversarial debate protocol.
    ///
    /// Returns (final_response, verified, confidence, rounds, extra_tokens_used)
    async fn run_adversarial_debate(
        &self,
        blue_role: &str,
        blue_content: &str,
    ) -> Result<(String, bool, f64, u32, u32, Vec<vex_core::Hash>), String> {
        // Create a Blue agent for role tracking
        let blue_agent = Agent::new(AgentConfig {
            name: self.payload.agent_id.clone(),
            role: blue_role.to_string(),
            max_depth: 1,
            spawn_shadow: true,
        });

        // Spawn shadow (Red) agent
        let shadow = ShadowAgent::new(&blue_agent, ShadowConfig::default());

        let mut debate = Debate::new(blue_agent.id, shadow.agent.id, blue_content);
        let mut consensus = Consensus::new(ConsensusProtocol::WeightedConfidence);
        let mut extra_tokens: u32 = 0;
        let mut rounds_completed: u32 = 0;
        let mut context_hashes: Vec<vex_core::Hash> = Vec::new();

        // --- Debate rounds ---
        for round_num in 1..=self.payload.max_debate_rounds {
            // Red agent challenges
            let mut challenge_prompt = shadow.challenge_prompt(blue_content);
            challenge_prompt.push_str(
                "\n\nIMPORTANT: Respond in valid JSON: \
                {\"is_challenge\": boolean, \"confidence\": float, \"reasoning\": \"string\"}. \
                If you agree with the claim, set is_challenge to false.",
            );

            let red_req = LlmRequest::with_role(&shadow.agent.config.role, &challenge_prompt);
            let red_resp = self
                .llm
                .complete(red_req)
                .await
                .map_err(|e| e.to_string())?;
            extra_tokens += red_resp.tokens_used.unwrap_or(0);

            // Parse Red's JSON response
            let (is_challenge, red_confidence, red_reasoning) =
                parse_challenge_response(&red_resp.content);
            context_hashes.push(ContextPacket::new(&red_resp.content).hash);

            // Blue rebuts if challenged
            let rebuttal = if is_challenge {
                let rebuttal_prompt = format!(
                    "Your response was challenged:\n\nOriginal: \"{}\"\n\nChallenge: \"{}\"\n\n\
                     Address these concerns or provide a revised response.",
                    blue_content, red_reasoning
                );
                let blue_req = LlmRequest::with_role(blue_role, &rebuttal_prompt);
                let blue_rebuttal = self
                    .llm
                    .complete(blue_req)
                    .await
                    .map_err(|e| e.to_string())?;
                extra_tokens += blue_rebuttal.tokens_used.unwrap_or(0);
                context_hashes.push(ContextPacket::new(&blue_rebuttal.content).hash);
                Some(blue_rebuttal.content)
            } else {
                None
            };

            debate.add_round(DebateRound {
                round: round_num,
                blue_claim: blue_content.to_string(),
                red_challenge: red_reasoning.clone(),
                blue_rebuttal: rebuttal,
            });

            consensus.add_vote(Vote {
                agent_id: shadow.agent.id,
                agrees: !is_challenge,
                confidence: red_confidence,
                reasoning: Some(red_reasoning),
            });

            rounds_completed = round_num;

            if !is_challenge {
                break; // Red is satisfied — no more rounds needed
            }
        }

        // --- Blue self-reflection vote ---
        let reflection_prompt = build_reflection_prompt(blue_content, &debate);
        let reflect_req = LlmRequest::with_role(blue_role, &reflection_prompt);

        if let Ok(reflect_resp) = self.llm.complete(reflect_req).await {
            extra_tokens += reflect_resp.tokens_used.unwrap_or(0);
            context_hashes.push(ContextPacket::new(&reflect_resp.content).hash);
            let (blue_agrees, blue_conf, blue_reasoning) =
                parse_vote_response(&reflect_resp.content, 0.75);
            consensus.add_vote(Vote {
                agent_id: blue_agent.id,
                agrees: blue_agrees,
                confidence: blue_conf,
                reasoning: Some(blue_reasoning),
            });
        }

        consensus.evaluate();
        let verified = consensus.reached;
        let confidence = consensus.confidence;

        // Pick final response: use last rebuttal if debate went adversarial, else original
        let final_response = if consensus.reached && consensus.decision == Some(true) {
            blue_content.to_string()
        } else if let Some(last) = debate.rounds.last() {
            last.blue_rebuttal
                .clone()
                .unwrap_or_else(|| blue_content.to_string())
        } else {
            blue_content.to_string()
        };

        Ok((
            final_response,
            verified,
            confidence,
            rounds_completed,
            extra_tokens,
            context_hashes,
        ))
    }

    /// Store an error result and return JobResult::Retry
    async fn store_error(&mut self, error: String) -> JobResult {
        let result = AgentJobResult {
            job_id: self.job_id,
            agent_id: self.payload.agent_id.clone(),
            prompt: self.payload.prompt.clone(),
            response: String::new(),
            tokens_used: None,
            completed_at: Utc::now(),
            success: false,
            error: Some(error.clone()),
            verified: false,
            confidence: 0.0,
            context_hash: None,
            debate_rounds: 0,
            merkle_root: None,
            new_context_id: None,
            evidence: None,
        };
        self.result_store.write().await.insert(self.job_id, result);
        JobResult::Retry(error)
    }
}

// ── Helpers ────────────────────────────────────────────────────────────────

fn parse_challenge_response(raw: &str) -> (bool, f64, String) {
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if let Ok(cr) = serde_json::from_str::<ChallengeResponse>(&raw[start..=end]) {
                return (cr.is_challenge, cr.confidence, cr.reasoning);
            }
        }
    }
    // Fallback: heuristic detection
    let is_challenge =
        raw.to_lowercase().contains("disagree") || raw.to_lowercase().contains("[challenge]");
    (is_challenge, 0.5, raw.chars().take(300).collect())
}

fn parse_vote_response(raw: &str, default_confidence: f64) -> (bool, f64, String) {
    if let Some(start) = raw.find('{') {
        if let Some(end) = raw.rfind('}') {
            if let Ok(vr) = serde_json::from_str::<VoteResponse>(&raw[start..=end]) {
                return (vr.agrees, vr.confidence, vr.reflection);
            }
        }
    }
    (
        true,
        default_confidence,
        "Reflection parse failed".to_string(),
    )
}

fn build_reflection_prompt(blue_content: &str, debate: &Debate) -> String {
    let mut prompt = format!(
        "You finished an adversarial debate about your original response.\n\n\
         Original: \"{}\"\n\nDebate rounds:\n",
        blue_content
    );
    for (i, round) in debate.rounds.iter().enumerate() {
        prompt.push_str(&format!(
            "Round {}: Red challenged: \"{}\" → Rebuttal: \"{}\"\n",
            i + 1,
            round.red_challenge,
            round.blue_rebuttal.as_deref().unwrap_or("N/A")
        ));
    }
    prompt.push_str(
        "\nDo you still stand by your original response? \
         Respond in JSON: {\"agrees\": boolean, \"confidence\": float, \"reflection\": \"string\"}.",
    );
    prompt
}
