//! # VEX Queue
//!
//! Async background worker queue for robust job processing.
//!
//! Features:
//! - Generic `Job` trait
//! - Pluggable backend (Memory, SQL, Redis)
//! - Worker pool with concurrency control
//! - Retry with exponential backoff

pub mod backend;
pub mod job;
pub mod memory;
pub mod worker;

pub use backend::QueueBackend;
pub use job::{Job, JobId, JobResult, JobStatus};
pub use memory::MemoryQueue;
pub use worker::{WorkerConfig, WorkerPool};
