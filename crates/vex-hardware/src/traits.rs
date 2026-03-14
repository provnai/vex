use anyhow::Result;
use async_trait::async_trait;

/// Abstraction for Hardware-Backed Identity (TPM, Secure Enclave, etc.)
#[async_trait]
pub trait HardwareIdentity: Send + Sync {
    /// Seal a secret (e.g. key seed) to the hardware.
    /// The `label` allows distinct secrets to be stored (e.g. "identity_seed").
    async fn seal(&self, label: &str, data: &[u8]) -> Result<Vec<u8>>;

    /// Unseal a secret using the hardware's private key.
    /// This should fail if the integrity of the machine state is compromised (if PCRs are checked).
    async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>>;

    /// Sign a handshake hash using the hardware identity key.
    async fn sign_handshake_hash(&self, hash: &[u8]) -> Result<[u8; 64]>;

    /// Perform a Diffie-Hellman exchange using the hardware-sealed static key.
    async fn dh(&self, remote_public_key: &[u8]) -> Result<[u8; 32]>;

    /// Generate a TPM Quote over the PCR state using the hardware identity key.
    /// The `nonce` is typically the `capsule_root` or a fresh challenge.
    async fn generate_quote(&self, nonce: &[u8]) -> Result<TpmQuote>;

    /// Retrieve the current PCR (Platform Configuration Register) values.
    /// Returns a map of PCR index to SHA-256 hash (hex string).
    async fn get_pcrs(&self, indices: &[u32]) -> Result<std::collections::HashMap<u32, String>>;

    /// Retrieve the public key (AID) associated with this hardware identity.
    async fn public_key(&self) -> Result<Vec<u8>>;

    /// Inject a sealed seed for use in signature and DH operations.
    /// This is mandatory for signing and DH in the 'Software-Seed + TPM-Seal' model.
    fn set_sealed_seed(&mut self, _sealed_seed: Vec<u8>) {}

    /// Inject the matching public key for this identity.
    /// This avoids unnecessary unsealing operations to retrieve the public key.
    fn set_public_key(&mut self, _pubkey: Vec<u8>) {}
}

/// Represents a TPM 2.0 Quote.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TpmQuote {
    /// The TPMS_ATTEST binary structure (signed message).
    pub message: Vec<u8>,
    /// The signature over the message.
    pub signature: Vec<u8>,
    /// Selected PCR values at the time of the quote (hex encoded).
    pub pcrs: std::collections::HashMap<u32, String>,
}

/// Abstraction for Network Monitoring (Process/Socket correlation)
pub trait NetworkWatchman: Send + Sync {
    /// Get a list of active connections for a specific process tree (PID + children).
    fn get_process_connections(&self, pid: u32) -> Result<Vec<ConnectionInfo>>;
}

#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    pub local_ip: String,
    pub local_port: u16,
    pub remote_ip: String,
    pub remote_port: u16,
    pub pid: u32,
    pub process_name: String,
}
