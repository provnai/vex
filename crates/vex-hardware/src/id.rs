use ed25519_dalek::{Signature, Signer, SigningKey};
use provn_sdk::{sign_claim, Claim};
use sha2::{Digest, Sha256};
use uuid::Uuid;
use zeroize::Zeroize;

/// AttestAgent represents a cryptographically-verified agent identity.
///
/// Alignment: This public key hash is used as the Uuid for vex-core agents,
/// providing a 1:1 mapping between Attest identity and VEX execution.
/// Alignment: This public key hash is used as the Uuid for vex-core agents,
/// providing a 1:1 mapping between Attest identity and VEX execution.
#[derive(Debug, Zeroize)]
#[zeroize(drop)]
pub struct AttestAgent {
    #[zeroize(skip)]
    pub signing_key: SigningKey,
    #[zeroize(skip)]
    pub id: String, // aid:ed25519:<hex_pubkey>
    pub sealed_seed: Option<Vec<u8>>,
}

impl AttestAgent {
    pub fn new() -> Self {
        let signing_key = provn_sdk::generate_keypair();
        let id = format!(
            "aid:ed25519:{}",
            hex::encode(signing_key.verifying_key().to_bytes())
        );

        Self {
            signing_key,
            id,
            sealed_seed: None,
        }
    }

    /// Restore an identity from a raw 32-byte seed
    pub fn from_seed(mut seed: [u8; 32]) -> Self {
        let signing_key = SigningKey::from_bytes(&seed);
        seed.zeroize();
        let id = format!(
            "aid:ed25519:{}",
            hex::encode(signing_key.verifying_key().to_bytes())
        );
        Self {
            signing_key,
            id,
            sealed_seed: None,
        }
    }

    /// Attach a sealed seed to this agent for TPM-bound operations
    pub fn with_sealed_seed(mut self, sealed_seed: Vec<u8>) -> Self {
        self.sealed_seed = Some(sealed_seed);
        self
    }

    /// Derives a VEX-compatible Uuid from the agent's public key
    pub fn to_vex_uuid(&self) -> Uuid {
        let mut hasher = Sha256::new();
        hasher.update(self.signing_key.verifying_key().as_bytes());
        let hash = hasher.finalize();
        Uuid::from_slice(&hash[..16]).expect("Hash slice must be 16 bytes")
    }

    /// Create and sign a provn-compatible claim
    pub fn sign_claim(&self, data: String) -> provn_sdk::SignedClaim {
        let claim = Claim::new(data);
        sign_claim(&claim, &self.signing_key).expect("Ecosystem signing failure")
    }

    /// Sign data returning a standard ed25519_dalek::Signature
    pub fn sign(&self, data: &[u8]) -> Signature {
        self.signing_key.sign(data)
    }
}

impl Default for AttestAgent {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_identity_alignment() {
        let agent = AttestAgent::new();
        let vex_id = agent.to_vex_uuid();

        println!("Attest ID: {}", agent.id);
        println!("VEX Context ID: {}", vex_id);

        // Ensure VEX ID is deterministic from public key
        assert_eq!(vex_id, agent.to_vex_uuid());
    }

    #[test]
    fn test_sdk_signing() {
        let agent = AttestAgent::new();
        let payload = "agent-did-something".to_string();
        let signed = agent.sign_claim(payload);
        println!("Signature: {}", signed.signature);
        assert!(provn_sdk::verify_claim(&signed).unwrap());
    }
}
