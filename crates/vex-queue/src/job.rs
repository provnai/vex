//! Job definitions

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use uuid::Uuid;

/// Job Identifier
pub type JobId = Uuid;

/// Job Status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum JobStatus {
    /// Waiting in queue
    Pending,
    /// Currently being processed
    Running,
    /// Successfully completed
    Completed,
    /// Failed (with retry count)
    Failed(u32),
    /// Permanently failed after max retries
    DeadLetter,
}

/// Generic Job Trait
#[async_trait::async_trait]
pub trait Job: Send + Sync + Debug {
    /// Job name/type
    fn name(&self) -> &str;

    /// Execute the job
    async fn execute(&mut self) -> JobResult;

    /// Max retries allowed
    fn max_retries(&self) -> u32 {
        3
    }

    /// Backoff strategy
    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Exponential {
            initial_secs: 1,
            multiplier: 2.0,
        }
    }
}

/// Result of job execution
#[derive(Debug)]
pub enum JobResult {
    /// Job succeeded
    Success,
    /// Job failed but should retry
    Retry(String),
    /// Job failed permanently
    Fatal(String),
}

/// Retry backoff strategy
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum BackoffStrategy {
    Constant { secs: u64 },
    Exponential { initial_secs: u64, multiplier: f64 },
}

impl BackoffStrategy {
    pub fn delay(&self, attempt: u32) -> std::time::Duration {
        match self {
            Self::Constant { secs } => std::time::Duration::from_secs(*secs),
            Self::Exponential {
                initial_secs,
                multiplier,
            } => {
                let secs = (*initial_secs as f64 * multiplier.powi(attempt as i32)) as u64;
                std::time::Duration::from_secs(secs)
            }
        }
    }
}

/// A persisted job entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobEntry {
    pub id: JobId,
    pub tenant_id: String,
    pub job_type: String,
    pub payload: serde_json::Value,
    pub status: JobStatus,
    pub created_at: DateTime<Utc>,
    pub run_at: DateTime<Utc>,
    pub attempts: u32,
    pub last_error: Option<String>,
}
