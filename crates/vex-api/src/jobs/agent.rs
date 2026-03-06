use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;
use vex_llm::LlmProvider;
use vex_persist::StorageBackend;
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
    #[serde(default)]
    pub capabilities: Vec<vex_llm::Capability>,
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
    pub orchestrator: Arc<vex_runtime::Orchestrator<dyn vex_llm::LlmProvider>>,
}

impl AgentExecutionJob {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        job_id: Uuid,
        payload: AgentJobPayload,
        llm: Arc<dyn LlmProvider>,
        result_store: JobResultStore,
        db: Arc<dyn StorageBackend>,
        anchor: Option<Arc<dyn vex_anchor::AnchorBackend>>,
        evolution_store: Arc<dyn vex_persist::EvolutionStore>,
        gate: Arc<dyn vex_runtime::Gate>,
        orchestrator: Arc<vex_runtime::Orchestrator<dyn vex_llm::LlmProvider>>,
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
            orchestrator,
        }
    }
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
            "Executing VEX agent job via Orchestrator"
        );

        let tenant_id = self.payload.tenant_id.as_deref().unwrap_or("default");

        // Use the unified Orchestrator for full cognitive cycle (includes Hardware Signing)
        let orchestration_result = match self
            .orchestrator
            .process(
                tenant_id,
                &self.payload.prompt,
                self.payload.capabilities.clone(),
            )
            .await
        {
            Ok(res) => res,
            Err(e) => {
                error!(job_id = %self.job_id, error = %e, "Orchestrator execution failed");
                return store_error(
                    &self.result_store,
                    self.job_id,
                    self.payload.agent_id.clone(),
                    self.payload.prompt.clone(),
                    e.to_string(),
                )
                .await;
            }
        };

        let result = AgentJobResult {
            job_id: self.job_id,
            agent_id: self.payload.agent_id.clone(),
            prompt: self.payload.prompt.clone(),
            response: orchestration_result.response,
            tokens_used: None, // Aggregated tokens not yet in OrchestrationResult
            completed_at: Utc::now(),
            success: true,
            error: None,
            verified: orchestration_result.confidence > 0.7, // Heuristic verification
            confidence: orchestration_result.confidence,
            context_hash: Some(hex::encode(orchestration_result.merkle_root.0)),
            debate_rounds: 0, // Handled internally by Orchestrator
            merkle_root: Some(hex::encode(orchestration_result.merkle_root.0)),
            new_context_id: None,
            evidence: None, // Evidence is stored in audit logs
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

// ── Helpers ────────────────────────────────────────────────────────────────

async fn store_error(
    result_store: &Arc<tokio::sync::RwLock<std::collections::HashMap<uuid::Uuid, AgentJobResult>>>,
    job_id: uuid::Uuid,
    agent_id: String,
    prompt: String,
    error: String,
) -> JobResult {
    let result = AgentJobResult {
        job_id,
        agent_id,
        prompt,
        response: String::new(),
        tokens_used: None,
        completed_at: chrono::Utc::now(),
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
    result_store.write().await.insert(job_id, result);
    JobResult::Retry(error)
}
