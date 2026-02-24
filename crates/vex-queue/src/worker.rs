//! Worker Pool for processing jobs

use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::backend::QueueBackend;
use crate::job::{Job, JobResult, JobStatus};

#[derive(Clone, Copy)]
pub struct WorkerConfig {
    pub max_concurrency: usize,
    pub poll_interval: Duration,
}

impl Default for WorkerConfig {
    fn default() -> Self {
        Self {
            max_concurrency: 5,
            poll_interval: Duration::from_millis(100),
        }
    }
}

use serde::de::DeserializeOwned;

use std::sync::RwLock;

pub struct WorkerPool<B: QueueBackend + ?Sized> {
    pub backend: Arc<B>,
    config: WorkerConfig,
    registry: Arc<JobRegistry>,
}

type JobFactory =
    Box<dyn Fn(serde_json::Value) -> Result<Box<dyn Job>, serde_json::Error> + Send + Sync>;

struct JobRegistry {
    factories: RwLock<std::collections::HashMap<String, JobFactory>>,
}

impl<B: QueueBackend + 'static> WorkerPool<B> {
    pub fn new(backend: B, config: WorkerConfig) -> Self {
        Self::new_with_arc(Arc::new(backend), config)
    }
}

impl<B: QueueBackend + ?Sized + 'static> WorkerPool<B> {
    /// Create new worker pool from existing Arc backend (supports dyn dispatch)
    pub fn new_with_arc(backend: Arc<B>, config: WorkerConfig) -> Self {
        Self {
            backend,
            config,
            registry: Arc::new(JobRegistry {
                factories: RwLock::new(std::collections::HashMap::new()),
            }),
        }
    }

    /// Register a job type handler
    pub fn register_job_type<J: Job + DeserializeOwned + 'static>(&self, name: &str) {
        let factory = Box::new(|payload: serde_json::Value| {
            let job: J = serde_json::from_value(payload)?;
            Ok(Box::new(job) as Box<dyn Job>)
        });

        self.registry
            .factories
            .write()
            .expect("Job registry RwLock poisoned")
            .insert(name.to_string(), factory);
    }

    /// Register a custom factory (useful for jobs with dependency injection)
    pub fn register_job_factory<F>(&self, name: &str, factory: F)
    where
        F: Fn(serde_json::Value) -> Box<dyn Job> + Send + Sync + 'static,
    {
        self.registry
            .factories
            .write()
            .expect("Job registry RwLock poisoned")
            .insert(
                name.to_string(),
                Box::new(move |payload| Ok(factory(payload))),
            );
    }

    pub async fn start(&self) {
        let semaphore = Arc::new(Semaphore::new(self.config.max_concurrency));

        info!(
            "Worker pool started with concurrency {}",
            self.config.max_concurrency
        );

        loop {
            if semaphore.available_permits() > 0 {
                match self.backend.dequeue().await {
                    Ok(Some(entry)) => {
                        let permit = semaphore
                            .clone()
                            .acquire_owned()
                            .await
                            .expect("Worker semaphore closed unexpectedly");
                        let backend = self.backend.clone();
                        let registry = self.registry.clone();

                        tokio::spawn(async move {
                            let job_opt = {
                                let factories = registry
                                    .factories
                                    .read()
                                    .expect("Job registry RwLock poisoned");
                                factories
                                    .get(&entry.job_type)
                                    .map(|f| f(entry.payload.clone()))
                            };

                            match job_opt {
                                Some(Ok(mut job)) => {
                                    info!("Processing job {} ({})", entry.id, entry.job_type);

                                    let result = job.execute().await;

                                    match result {
                                        JobResult::Success(value) => {
                                            if let Some(val) = value {
                                                let _ = backend.set_result(entry.id, val).await;
                                            }
                                            let _ = backend
                                                .update_status(
                                                    entry.id,
                                                    JobStatus::Completed,
                                                    None,
                                                    None,
                                                )
                                                .await;
                                        }
                                        JobResult::Retry(e) => {
                                            let delay =
                                                job.backoff_strategy().delay(entry.attempts);
                                            let delay_secs = delay.as_secs();

                                            info!(
                                                job_id = %entry.id,
                                                attempt = entry.attempts + 1,
                                                delay_secs = delay_secs,
                                                "Job failed, scheduling retry with backoff"
                                            );

                                            let _ = backend
                                                .update_status(
                                                    entry.id,
                                                    JobStatus::Failed(entry.attempts + 1),
                                                    Some(e),
                                                    Some(delay_secs),
                                                )
                                                .await;
                                        }
                                        JobResult::Fatal(e) => {
                                            let _ = backend
                                                .update_status(
                                                    entry.id,
                                                    JobStatus::DeadLetter,
                                                    Some(e),
                                                    None,
                                                )
                                                .await;
                                        }
                                    }
                                }
                                Some(Err(e)) => {
                                    error!(job_id = %entry.id, error = %e, "Job payload deserialization failed");
                                    let _ = backend
                                        .update_status(
                                            entry.id,
                                            JobStatus::DeadLetter,
                                            Some(e.to_string()),
                                            None,
                                        )
                                        .await;
                                }
                                None => {
                                    warn!("No handler registered for job type: {}", entry.job_type);
                                    let _ = backend
                                        .update_status(
                                            entry.id,
                                            JobStatus::DeadLetter,
                                            Some(format!("No handler for {}", entry.job_type)),
                                            None,
                                        )
                                        .await;
                                }
                            }

                            drop(permit);
                        });
                    }
                    Ok(None) => {
                        // Queue empty
                        tokio::time::sleep(self.config.poll_interval).await;
                    }
                    Err(e) => {
                        error!("Queue error: {}", e);
                        tokio::time::sleep(Duration::from_secs(1)).await;
                    }
                }
            } else {
                // Wait for permit availability
                tokio::time::sleep(Duration::from_millis(50)).await;
            }
        }
    }
}
