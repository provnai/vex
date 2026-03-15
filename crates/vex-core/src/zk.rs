//! # VEX ZK Bridge
//!
//! Provides the trait-based interface for Zero-Knowledge proof verification.
//! This allows vex-core to remain decoupled from heavy ZK libraries like Plonky3
//! while still supporting "Shadow Intent" verification.

use serde_json::Value;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ZkError {
    #[error("Verification failed: {0}")]
    VerificationFailed(String),
    #[error("Invalid proof format: {0}")]
    InvalidFormat(String),
    #[error("Missing public inputs: {0}")]
    MissingMetadata(String),
}

/// Interface for ZK-STARK verification.
/// Implementations are provided by downstream crates (e.g., attest-rs).
pub trait ZkVerifier {
    /// Verifies a STARK proof against a commitment and public inputs.
    ///
    /// # Arguments
    /// * `commitment_hash` - The root hash the proof must bind to.
    /// * `stark_proof_b64` - Base64 encoded STARK proof.
    /// * `public_inputs` - JSON values representing the non-private data.
    fn verify_stark(
        &self,
        commitment_hash: &str,
        stark_proof_b64: &str,
        public_inputs: &Value,
    ) -> Result<bool, ZkError>;
}
