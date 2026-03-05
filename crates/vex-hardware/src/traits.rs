use async_trait::async_trait;
use anyhow::Result;

/// Abstraction for Hardware-Backed Identity (TPM, Secure Enclave, etc.)
#[async_trait]
pub trait HardwareIdentity: Send + Sync {
    /// Seal a secret (e.g. key seed) to the hardware.
    /// The `label` allows distinct secrets to be stored (e.g. "identity_seed").
    async fn seal(&self, label: &str, data: &[u8]) -> Result<Vec<u8>>;

    /// Unseal a secret using the hardware's private key.
    /// This should fail if the integrity of the machine state is compromised (if PCRs are checked).
    async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>>;
}
