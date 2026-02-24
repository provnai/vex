//! In-memory queue implementation with priority scheduling

use async_trait::async_trait;
use chrono::{DateTime, Duration, Utc};
use std::cmp::Ordering;
use std::collections::{BinaryHeap, HashMap};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::backend::{QueueBackend, QueueError};
use crate::job::{JobEntry, JobStatus};

/// Priority entry for the heap - orders by run_at time (earliest first)
#[derive(Debug, Clone, Eq, PartialEq)]
struct PriorityEntry {
    run_at: DateTime<Utc>,
    id: Uuid,
}

impl Ord for PriorityEntry {
    fn cmp(&self, other: &Self) -> Ordering {
        // Reverse order: earlier run_at = higher priority
        other.run_at.cmp(&self.run_at)
    }
}

impl PartialOrd for PriorityEntry {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Debug, Default)]
pub struct MemoryQueue {
    jobs: Arc<RwLock<HashMap<Uuid, JobEntry>>>,
    queue: Arc<RwLock<BinaryHeap<PriorityEntry>>>,
}

impl MemoryQueue {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl QueueBackend for MemoryQueue {
    async fn enqueue(
        &self,
        tenant_id: &str,
        job_type: &str,
        payload: serde_json::Value,
        delay_secs: Option<u64>,
    ) -> Result<Uuid, QueueError> {
        let id = Uuid::new_v4();
        let now = Utc::now();
        let run_at = now + Duration::seconds(delay_secs.unwrap_or(0) as i64);

        let entry = JobEntry {
            id,
            tenant_id: tenant_id.to_string(),
            job_type: job_type.to_string(),
            payload,
            status: JobStatus::Pending,
            created_at: now,
            run_at,
            attempts: 0,
            last_error: None,
            result: None,
        };

        let mut jobs = self.jobs.write().await;
        jobs.insert(id, entry);

        let mut queue = self.queue.write().await;
        queue.push(PriorityEntry { run_at, id });

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<JobEntry>, QueueError> {
        let mut queue = self.queue.write().await;
        let now = Utc::now();

        // Peek at the earliest job
        if let Some(entry) = queue.peek() {
            if entry.run_at <= now {
                // Job is ready - pop it
                let entry = queue.pop().unwrap();
                let mut jobs = self.jobs.write().await;

                if let Some(job) = jobs.get_mut(&entry.id) {
                    // Only process if still pending
                    if job.status == JobStatus::Pending {
                        job.status = JobStatus::Running;
                        return Ok(Some(job.clone()));
                    }
                }
            }
        }

        Ok(None)
    }

    async fn update_status(
        &self,
        id: Uuid,
        status: JobStatus,
        error: Option<String>,
        delay_secs: Option<u64>,
    ) -> Result<(), QueueError> {
        let mut jobs = self.jobs.write().await;

        if let Some(job) = jobs.get_mut(&id) {
            job.status = status;
            job.last_error = error;
            job.attempts += if matches!(status, JobStatus::Failed(_)) {
                1
            } else {
                0
            };

            if let JobStatus::Failed(retry_count) = status {
                // Use provided delay or fall back to exponential backoff
                let backoff_secs = delay_secs.unwrap_or_else(|| {
                    2_u64.pow(retry_count.min(6)) // Default: exponential, cap at ~1 min
                });
                let run_at = Utc::now() + Duration::seconds(backoff_secs as i64);
                job.run_at = run_at;
                job.status = JobStatus::Pending; // Reset to pending for retry

                tracing::debug!(
                    job_id = %id,
                    retry_count = retry_count,
                    delay_secs = backoff_secs,
                    "Re-queuing job with backoff"
                );

                let mut queue = self.queue.write().await;
                queue.push(PriorityEntry { run_at, id });
            }
        }

        Ok(())
    }

    async fn get_status(&self, tenant_id: &str, id: Uuid) -> Result<JobStatus, QueueError> {
        let jobs = self.jobs.read().await;
        let job = jobs.get(&id).ok_or(QueueError::NotFound)?;

        if job.tenant_id != tenant_id {
            return Err(QueueError::NotFound);
        }

        Ok(job.status)
    }

    async fn get_job(&self, tenant_id: &str, id: Uuid) -> Result<JobEntry, QueueError> {
        let jobs = self.jobs.read().await;
        let job = jobs.get(&id).ok_or(QueueError::NotFound)?;

        if job.tenant_id != tenant_id {
            return Err(QueueError::NotFound);
        }

        Ok(job.clone())
    }

    async fn set_result(&self, id: Uuid, result: serde_json::Value) -> Result<(), QueueError> {
        let mut jobs = self.jobs.write().await;
        if let Some(job) = jobs.get_mut(&id) {
            job.result = Some(result);
            Ok(())
        } else {
            Err(QueueError::NotFound)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_enqueue_dequeue() {
        let queue = MemoryQueue::new();
        let payload = json!({ "foo": "bar" });

        // Enqueue
        let id = queue
            .enqueue("test-tenant", "test_job", payload.clone(), None)
            .await
            .unwrap();

        let status = queue.get_status("test-tenant", id).await.unwrap();
        assert_eq!(status, JobStatus::Pending);

        // Dequeue
        let job = queue.dequeue().await.unwrap().expect("Should have job");
        assert_eq!(job.id, id);
        assert_eq!(job.job_type, "test_job");
        assert_eq!(job.status, JobStatus::Running);

        // Dequeue empty
        let empty = queue.dequeue().await.unwrap();
        assert!(empty.is_none());
    }

    #[tokio::test]
    async fn test_delayed_job() {
        let queue = MemoryQueue::new();
        let payload = json!({});

        // Enqueue with delay
        let id = queue
            .enqueue("test-tenant", "delayed", payload, Some(1))
            .await
            .unwrap();

        // Should be none immediately
        let job = queue.dequeue().await.unwrap();
        assert!(job.is_none());

        // Wait
        tokio::time::sleep(std::time::Duration::from_millis(1100)).await;

        // Should be available (Note: MemoryQueue checks run_at > now)
        let job = queue
            .dequeue()
            .await
            .unwrap()
            .expect("Should have delayed job");
        assert_eq!(job.id, id);
    }
}
