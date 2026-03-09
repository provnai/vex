use crate::client::{AuthorityClient, ChoraResponse};
use serde::Serialize;
use vex_core::segment::{Capsule, IdentityData, IntentData, WitnessData};

/// The Bridge manages the relationship between local agent intent
/// and external authority validation.
pub struct AuthorityBridge {
    pub client: Box<dyn AuthorityClient + Send + Sync>,
}

impl AuthorityBridge {
    pub fn new(client: Box<dyn AuthorityClient + Send + Sync>) -> Self {
        Self { client }
    }

    /// Perform the v0.2.0 Authority Handshake
    /// Bridges the cognitive Intent to a Witness-signed Capsule.
    pub async fn perform_handshake(&self, intent: IntentData) -> Result<Capsule, String> {
        // 1. Canonicalize Intent for CHORA witness
        let payload = Self::canonicalize(&intent)?;

        // 2. Request Attestation from Authority Layer
        let response: ChoraResponse = self.client.request_attestation(&payload).await?;

        // 3. Construct Composite Capsule
        // Note: IdentityData is currently mocked with dummy values until Attest-RS wiring is complete.
        let identity = IdentityData {
            agent: "mock-hardware-id-01".to_string(),
            tpm: "mock-tpm-quote-base64".to_string(),
        };

        // 4. Mock WitnessData for v1.1.0/Capsule v1 Parity
        let witness = WitnessData {
            chora_node_id: "node-0".to_string(),
            receipt_hash: hex::encode(vec![0u8; 32]),
            timestamp: "2024-03-09T10:00:00Z".to_string(),
        };

        Ok(Capsule {
            intent,
            authority: response.authority,
            identity,
            witness,
            chora_signature: response.signature,
        })
    }

    /// Canonicalize a payload for CHORA compliance (RFC 8785)
    pub fn canonicalize<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
        serde_jcs::to_vec(payload).map_err(|e| format!("JCS Canonicalization failed: {}", e))
    }
}
