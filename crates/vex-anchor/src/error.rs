//! Error types for anchoring operations

use thiserror::Error;

/// Errors that can occur during anchoring operations
#[derive(Debug, Error)]
pub enum AnchorError {
    /// Backend is not available or misconfigured
    #[error("Backend unavailable: {0}")]
    BackendUnavailable(String),

    /// Failed to write anchor data
    #[error("Write failed: {0}")]
    WriteFailed(String),

    /// Failed to read anchor data
    #[error("Read failed: {0}")]
    ReadFailed(String),

    /// Anchor not found during verification
    #[error("Anchor not found: {0}")]
    NotFound(String),

    /// Anchor verification failed (tampering detected)
    #[error("Verification failed: {0}")]
    VerificationFailed(String),

    /// Serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// I/O error
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Git operation failed
    #[error("Git error: {0}")]
    Git(String),

    /// Blockchain/network error
    #[error("Network error: {0}")]
    Network(String),
}
