use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tracing::{error, info};
use uuid::Uuid;
use vex_llm::{LlmProvider, LlmRequest};
use vex_queue::job::BackoffStrategy;
use vex_queue::{Job, JobResult};

/// Payload for agent execution job
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentJobPayload {
    pub agent_id: String,
    pub prompt: String,
    pub context_id: Option<String>,
}

/// Result of an agent execution job
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
}

impl AgentExecutionJob {
    pub fn new(
        job_id: Uuid,
        payload: AgentJobPayload,
        llm: Arc<dyn LlmProvider>,
        result_store: JobResultStore,
    ) -> Self {
        Self {
            job_id,
            payload,
            llm,
            result_store,
        }
    }
}

#[async_trait]
impl Job for AgentExecutionJob {
    fn name(&self) -> &str {
        "agent_execution"
    }

    async fn execute(&mut self) -> JobResult {
        info!(job_id = %self.job_id, agent_id = %self.payload.agent_id, "Executing agent job");

        let request = LlmRequest::with_role("You are a helpful VEX agent.", &self.payload.prompt);

        match self.llm.complete(request).await {
            Ok(response) => {
                let tokens = response.tokens_used.unwrap_or(0) as u64;
                vex_llm::global_metrics().record_llm_call(tokens, false);

                info!(
                    job_id = %self.job_id,
                    agent_id = %self.payload.agent_id,
                    response_len = response.content.len(),
                    tokens = tokens,
                    "Agent job completed successfully"
                );

                // Store the result
                let result = AgentJobResult {
                    job_id: self.job_id,
                    agent_id: self.payload.agent_id.clone(),
                    prompt: self.payload.prompt.clone(),
                    response: response.content,
                    tokens_used: response.tokens_used,
                    completed_at: Utc::now(),
                    success: true,
                    error: None,
                };

                self.result_store
                    .write()
                    .await
                    .insert(self.job_id, result.clone());

                JobResult::Success(Some(serde_json::to_value(&result).unwrap()))
            }
            Err(e) => {
                vex_llm::global_metrics().record_llm_call(0, true);
                error!(job_id = %self.job_id, error = %e, "LLM call failed");

                // Store error result
                let result = AgentJobResult {
                    job_id: self.job_id,
                    agent_id: self.payload.agent_id.clone(),
                    prompt: self.payload.prompt.clone(),
                    response: String::new(),
                    tokens_used: None,
                    completed_at: Utc::now(),
                    success: false,
                    error: Some(e.to_string()),
                };

                self.result_store.write().await.insert(self.job_id, result);

                JobResult::Retry(e.to_string())
            }
        }
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
