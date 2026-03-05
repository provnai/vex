use ed25519_dalek::{Signature, Signer, SigningKey};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// AttestAgent represents a cryptographically-verified agent identity.
#[derive(Debug)]
pub struct AttestAgent {
    pub signing_key: SigningKey,
    pub id: String, // aid:ed25519:<hex_pubkey>
}

impl AttestAgent {
    /// Restore an identity from a raw 32-byte seed
    pub fn from_seed(seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        let id = format!("aid:ed25519:{}", hex::encode(signing_key.verifying_key().to_bytes()));
        Self { signing_key, id }
    }

    /// Derives a VEX-compatible Uuid from the agent's public key
    pub fn to_vex_uuid(&self) -> Uuid {
        let mut hasher = Sha256::new();
        hasher.update(self.signing_key.verifying_key().as_bytes());
        let hash = hasher.finalize();
        Uuid::from_slice(&hash[..16]).expect("Hash slice must be 16 bytes")
    }

    /// Sign data returning a standard ed25519_dalek::Signature
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }
}
