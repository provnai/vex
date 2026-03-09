use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use vex_core::segment::AuthorityData;

/// Response from the CHORA Authority.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChoraResponse {
    pub authority: AuthorityData,
    pub signature: String,
}

/// Trait for external authority clients.
/// This ensures VEX remains neutral and can support multiple witness providers.
#[async_trait]
pub trait AuthorityClient {
    async fn request_attestation(&self, payload: &[u8]) -> Result<ChoraResponse, String>;
    async fn verify_witness_signature(
        &self,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<bool, String>;
}

/// A Mock Authority Client for test/dev environments.
/// Generates deterministic signatures based on a test key.
pub struct MockChoraClient;

#[async_trait]
impl AuthorityClient for MockChoraClient {
    async fn request_attestation(&self, payload: &[u8]) -> Result<ChoraResponse, String> {
        use ed25519_dalek::{Signer, SigningKey};
        use sha2::{Digest, Sha256};

        // SHA-256 for witness_receipt
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let hash = hasher.finalize();
        let _witness_receipt = hex::encode(hash);

        let authority = AuthorityData {
            capsule_id: "chora-mock-id".into(),
            outcome: "ALLOW".into(),
            reason_code: "MOCK_OK".into(),
            nonce: 42,
            trace_root: [0u8; 32], // Mocked trace root
        };

        // Generate mock signature
        let signing_key = SigningKey::from_bytes(&[0u8; 32]);
        let sig = signing_key.sign(payload);
        let signature = hex::encode(sig.to_bytes());

        Ok(ChoraResponse {
            authority,
            signature,
        })
    }

    async fn verify_witness_signature(
        &self,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<bool, String> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};
        let verifying_key = VerifyingKey::from_bytes(&[0u8; 32]).map_err(|e| e.to_string())?;
        let sig = Signature::from_bytes(signature.try_into().map_err(|_| "Invalid sig length")?);
        Ok(verifying_key.verify(payload, &sig).is_ok())
    }
}
