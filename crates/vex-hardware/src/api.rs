use crate::id::AttestAgent;
use crate::tpm::{create_identity_provider, HardwareIdentity};
use anyhow::Result;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum HardwareError {
    #[error("No TPM found or initialization failed: {0}")]
    NoTpmFound(String),
    #[error("Hardware operation failed: {0}")]
    OperationFailed(String),
}

pub struct HardwareKeystore {
    provider: Box<dyn HardwareIdentity>,
}

impl HardwareKeystore {
    /// Initialize connection to the TPM (or CNG on Windows)
    pub async fn new() -> Result<Self, HardwareError> {
        let allow_fallback = std::env::var("VEX_HARDWARE_ATTESTATION")
            .map(|v| v != "true")
            .unwrap_or(true);
            
        let provider = create_identity_provider(allow_fallback)?;
        Ok(Self { provider })
    }

    /// Helper to seed identity for external persistence (VEX-persist)
    pub async fn seal_identity(&self, seed: &[u8; 32]) -> Result<Vec<u8>, HardwareError> {
        self.provider.seal("identity_seed", seed).await.map_err(|e| HardwareError::OperationFailed(e.to_string()))
    }

    /// Get the Unsealed Identity for real-time signing from a persisted hardware blob
    pub async fn get_identity(&self, encrypted_blob: &[u8]) -> Result<AgentIdentity, HardwareError> {
        let seed = self.provider.unseal(encrypted_blob).await
            .map_err(|e| HardwareError::OperationFailed(e.to_string()))?;
            
        let seed_array: [u8; 32] = seed.try_into()
            .map_err(|_| HardwareError::OperationFailed("Unsealed seed is not 32 bytes".into()))?;
            
        let agent = AttestAgent::from_seed(seed_array);
        
        Ok(AgentIdentity {
            agent_id: agent.to_vex_uuid().to_string(),
            inner: agent,
        })
    }
}

pub struct AgentIdentity {
    pub agent_id: String,
    inner: AttestAgent,
}

impl AgentIdentity {
    /// Generate an Ed25519 signature over the provided deterministic bytes
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.inner.sign(message).to_bytes().to_vec()
    }
}
