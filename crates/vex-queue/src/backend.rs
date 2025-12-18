//! Queue Backend Trait

use crate::job::{JobEntry, JobStatus};
use async_trait::async_trait;
use uuid::Uuid;

#[derive(Debug, thiserror::Error)]
pub enum QueueError {
    #[error("Backend error: {0}")]
    Backend(String),
    #[error("Job not found")]
    NotFound,
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[async_trait]
pub trait QueueBackend: Send + Sync {
    /// Enqueue a job payload for a specific tenant
    async fn enqueue(
        &self,
        tenant_id: &str,
        job_type: &str,
        payload: serde_json::Value,
        delay_secs: Option<u64>,
    ) -> Result<Uuid, QueueError>;

    /// Pull next available job
    async fn dequeue(&self) -> Result<Option<JobEntry>, QueueError>;

    /// Update job status (ack/nack)
    /// `delay_secs` is used for retries - how long to wait before the job is available again
    async fn update_status(
        &self,
        id: Uuid,
        status: JobStatus,
        error: Option<String>,
        delay_secs: Option<u64>,
    ) -> Result<(), QueueError>;

    /// Get job status
    async fn get_status(&self, id: Uuid) -> Result<JobStatus, QueueError>;
}
