//! Unit tests for vex-queue worker pool

use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use vex_queue::backend::QueueBackend;
use vex_queue::job::{BackoffStrategy, Job, JobResult};
use vex_queue::memory::MemoryQueue;
use vex_queue::worker::WorkerConfig;

/// A test job that counts executions
#[derive(Debug)]
#[allow(dead_code)]
struct CounterJob {
    counter: Arc<AtomicU32>,
    should_fail: bool,
    fail_times: u32,
}

#[allow(dead_code)]
impl CounterJob {
    fn new(counter: Arc<AtomicU32>) -> Self {
        Self {
            counter,
            should_fail: false,
            fail_times: 0,
        }
    }

    fn failing(counter: Arc<AtomicU32>, fail_times: u32) -> Self {
        Self {
            counter,
            should_fail: true,
            fail_times,
        }
    }
}

#[async_trait::async_trait]
impl Job for CounterJob {
    fn name(&self) -> &str {
        "counter_job"
    }

    async fn execute(&mut self) -> JobResult {
        let count = self.counter.fetch_add(1, Ordering::SeqCst);

        if self.should_fail && count < self.fail_times {
            JobResult::Retry(format!("Failing on attempt {}", count + 1))
        } else {
            JobResult::Success(None)
        }
    }

    fn max_retries(&self) -> u32 {
        5
    }

    fn backoff_strategy(&self) -> BackoffStrategy {
        BackoffStrategy::Constant { secs: 0 } // No delay for tests
    }
}

/// A job that always fails fatally
#[derive(Debug)]
#[allow(dead_code)]
struct FatalJob;

#[async_trait::async_trait]
impl Job for FatalJob {
    fn name(&self) -> &str {
        "fatal_job"
    }

    async fn execute(&mut self) -> JobResult {
        JobResult::Fatal("This job always fails".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[tokio::test]
    async fn test_worker_config_defaults() {
        let config = WorkerConfig::default();
        assert!(config.max_concurrency > 0);
        assert!(config.poll_interval.as_millis() > 0);
    }

    #[tokio::test]
    async fn test_memory_queue_enqueue_dequeue() {
        let queue = MemoryQueue::new();
        let tenant_id = "test-tenant";

        // Enqueue a job
        let id = queue
            .enqueue(tenant_id, "test_job", json!({"key": "value"}), None)
            .await
            .unwrap();

        // Dequeue it
        let job = queue.dequeue().await.unwrap();
        assert!(job.is_some());

        let job = job.unwrap();
        assert_eq!(job.id, id);
        assert_eq!(job.tenant_id, tenant_id);
        assert_eq!(job.job_type, "test_job");
        assert_eq!(job.payload["key"], "value");
    }

    #[tokio::test]
    async fn test_memory_queue_fifo_ordering() {
        let queue = MemoryQueue::new();
        let t1 = "tenant-1";

        let id1 = queue.enqueue(t1, "job1", json!({}), None).await.unwrap();
        let id2 = queue.enqueue(t1, "job2", json!({}), None).await.unwrap();

        // First in, first out
        let job1 = queue.dequeue().await.unwrap().unwrap();
        let job2 = queue.dequeue().await.unwrap().unwrap();

        assert_eq!(job1.id, id1);
        assert_eq!(job2.id, id2);
    }

    #[tokio::test]
    async fn test_backoff_strategy_constant() {
        let strategy = BackoffStrategy::Constant { secs: 5 };
        assert_eq!(strategy.delay(0).as_secs(), 5);
        assert_eq!(strategy.delay(3).as_secs(), 5);
        assert_eq!(strategy.delay(10).as_secs(), 5);
    }

    #[tokio::test]
    async fn test_backoff_strategy_exponential() {
        let strategy = BackoffStrategy::Exponential {
            initial_secs: 1,
            multiplier: 2.0,
        };
        assert_eq!(strategy.delay(0).as_secs(), 1); // 1 * 2^0 = 1
        assert_eq!(strategy.delay(1).as_secs(), 2); // 1 * 2^1 = 2
        assert_eq!(strategy.delay(2).as_secs(), 4); // 1 * 2^2 = 4
        assert_eq!(strategy.delay(3).as_secs(), 8); // 1 * 2^3 = 8
    }

    #[tokio::test]
    async fn test_job_result_variants() {
        // Just ensuring the enum variants exist and can be matched
        let success = JobResult::Success(None);
        let retry = JobResult::Retry("error".to_string());
        let fatal = JobResult::Fatal("error".to_string());

        assert!(matches!(success, JobResult::Success(_)));
        assert!(matches!(retry, JobResult::Retry(_)));
        assert!(matches!(fatal, JobResult::Fatal(_)));
    }

    #[tokio::test]
    async fn test_delayed_job_not_immediately_available() {
        let queue = MemoryQueue::new();
        let t1 = "tenant-1";

        // Enqueue with 10 second delay
        let _id = queue
            .enqueue(t1, "delayed", json!({}), Some(10))
            .await
            .unwrap();

        // Should not be dequeue-able immediately
        let job = queue.dequeue().await.unwrap();
        assert!(
            job.is_none(),
            "Delayed job should not be immediately available"
        );
    }

    #[tokio::test]
    async fn test_empty_queue_returns_none() {
        let queue = MemoryQueue::new();
        let job = queue.dequeue().await.unwrap();
        assert!(job.is_none());
    }
}
