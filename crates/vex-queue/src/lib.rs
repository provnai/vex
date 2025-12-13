//! # VEX Queue
//!
//! Async background worker queue for robust job processing.
//!
//! Features:
//! - Generic `Job` trait
//! - Pluggable backend (Memory, SQL, Redis)
//! - Worker pool with concurrency control
//! - Retry with exponential backoff

pub mod job;
pub mod worker;
pub mod backend;
pub mod memory;

pub use job::{Job, JobStatus, JobId, JobResult};
pub use worker::{WorkerPool, WorkerConfig};
pub use backend::QueueBackend;
pub use memory::MemoryQueue;
