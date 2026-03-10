use crate::client::{AuthorityClient, ChoraResponse};
use serde::Serialize;
use std::sync::Arc;
use vex_core::segment::{Capsule, IdentityData, IntentData, WitnessData};
use vex_hardware::api::AgentIdentity;

/// The Bridge manages the relationship between local agent intent
/// and external authority validation.
pub struct AuthorityBridge {
    pub client: Box<dyn AuthorityClient>,
    /// Optional hardware identity — when present, IdentityData in the capsule
    /// carries the real agent_id and a hardware-signed attestation.
    pub identity: Option<Arc<AgentIdentity>>,
}

impl AuthorityBridge {
    pub fn new(client: Box<dyn AuthorityClient>) -> Self {
        Self {
            client,
            identity: None,
        }
    }

    /// Attach a hardware identity so the bridge can produce real IdentityData.
    pub fn with_identity(mut self, identity: Arc<AgentIdentity>) -> Self {
        self.identity = Some(identity);
        self
    }

    /// Perform the v0.2.0 Authority Handshake
    /// Bridges the cognitive Intent to a Witness-signed Capsule.
    pub async fn perform_handshake(&self, intent: IntentData) -> Result<Capsule, String> {
        // 1. Canonicalize Intent for CHORA witness (RFC 8785 JCS)
        let payload = Self::canonicalize(&intent)?;

        // 2. Request Attestation from Authority Layer
        let response: ChoraResponse = self.client.request_attestation(&payload).await?;

        // 3. Build IdentityData from hardware when available, mock otherwise.
        let identity = if let Some(hw) = &self.identity {
            // Derive a compact attestation token by signing the authority capsule_id.
            // This proves this hardware agent authorized this specific CHORA decision.
            let attest_token = hw.sign(response.authority.capsule_id.as_bytes());
            IdentityData {
                agent: hw.agent_id.clone(),
                tpm: hex::encode(&attest_token),
            }
        } else {
            tracing::warn!(
                "AuthorityBridge: No hardware identity attached. Using mock IdentityData. \
                 Call .with_identity() to enable real attestation."
            );
            IdentityData {
                agent: "mock-hardware-id-01".to_string(),
                tpm: "mock-tpm-quote-base64".to_string(),
            }
        };

        // 4. WitnessData — populated from the live CHORA response.
        // receipt_hash: CHORA's signature over the capsule (custody proof).
        // timestamp: RFC3339 UTC of this handshake — never hardcoded.
        let now = chrono::Utc::now().to_rfc3339();
        let witness = WitnessData {
            chora_node_id: response.authority.capsule_id.clone(),
            receipt_hash: response.signature.clone(),
            timestamp: now,
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
