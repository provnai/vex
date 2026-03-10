use crate::id::AttestAgent;
use crate::tpm::{create_identity_provider, HardwareIdentity};
use anyhow::Result;
use thiserror::Error;
use zeroize::Zeroize;

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

impl std::fmt::Debug for HardwareKeystore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("HardwareKeystore").finish_non_exhaustive()
    }
}

impl HardwareKeystore {
    /// Initialize connection to the TPM (or CNG on Windows)
    pub async fn new() -> Result<Self, HardwareError> {
        let dev_mode = std::env::var("VEX_DEV_MODE")
            .map(|v| v == "1")
            .unwrap_or(false);
        let allow_fallback = std::env::var("VEX_HARDWARE_ATTESTATION")
            .map(|v| v != "true")
            .unwrap_or(true);

        // Force stub identity in dev mode if fallback is allowed, bypassing potential TPM malfunctions
        if dev_mode && allow_fallback {
            return Ok(Self {
                provider: Box::new(crate::tpm::StubIdentity::default()),
            });
        }

        let provider = create_identity_provider(allow_fallback);
        Ok(Self { provider })
    }

    /// Helper to seed identity for external persistence (VEX-persist)
    pub async fn seal_identity(&self, seed: &[u8; 32]) -> Result<Vec<u8>, HardwareError> {
        self.provider
            .seal("identity_seed", seed)
            .await
            .map_err(|e| HardwareError::OperationFailed(e.to_string()))
    }

    /// Get the Unsealed Identity for real-time signing from a persisted hardware blob
    pub async fn get_identity(
        &self,
        encrypted_blob: &[u8],
    ) -> Result<AgentIdentity, HardwareError> {
        let seed = self
            .provider
            .unseal(encrypted_blob)
            .await
            .map_err(|e| HardwareError::OperationFailed(e.to_string()))?;

        let mut seed_array: [u8; 32] = seed
            .try_into()
            .map_err(|_| HardwareError::OperationFailed("Unsealed seed is not 32 bytes".into()))?;

        let agent = crate::id::AttestAgent::from_seed(seed_array);
        seed_array.zeroize();

        Ok(AgentIdentity {
            agent_id: agent.to_vex_uuid().to_string(),
            inner: agent,
        })
    }
}

#[derive(Debug, Clone)]
pub struct AgentIdentity {
    pub agent_id: String,
    inner: AttestAgent,
}

impl Default for AgentIdentity {
    fn default() -> Self {
        let agent = crate::id::AttestAgent::new();
        Self {
            agent_id: agent.to_vex_uuid().to_string(),
            inner: agent,
        }
    }
}

impl AgentIdentity {
    /// Create a fresh hardware identity (for simulation or new agents)
    pub fn new() -> Self {
        Self::default()
    }

    /// Generate an Ed25519 signature over the provided deterministic bytes
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.inner.sign(message).to_bytes().to_vec()
    }
}
