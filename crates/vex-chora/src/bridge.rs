use crate::client::{AuthorityClient, ChoraResponse};
use serde::Serialize;
use std::sync::Arc;
use vex_core::segment::{Capsule, CryptoData, IdentityData, IntentData, WitnessData};
use vex_hardware::api::AgentIdentity;

/// The Bridge manages the relationship between local agent intent
/// and external authority validation.
#[derive(Debug, Clone)]
pub struct AuthorityBridge {
    pub client: Arc<dyn AuthorityClient>,
    /// Optional hardware identity — when present, IdentityData in the capsule
    /// carries the real agent_id and a hardware-signed attestation.
    pub identity: Option<Arc<AgentIdentity>>,
}

impl AuthorityBridge {
    pub fn new(client: Arc<dyn AuthorityClient>) -> Self {
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

    /// Perform the v0.1.0 "Hardened" Authority Handshake
    /// Bridges the cognitive Intent to a Witness-signed Capsule.
    pub async fn perform_handshake(&self, intent: IntentData) -> Result<Capsule, String> {
        // 1. Canonicalize Intent for CHORA witness (RFC 8785 JCS)
        let payload = Self::canonicalize(&intent)?;

        // 2. Request Attestation from Authority Layer
        let response: ChoraResponse = self.client.request_attestation(&payload).await?;

        // 3. Build IdentityData from hardware when available.
        let identity = if let Some(hw) = &self.identity {
            IdentityData {
                aid: hw.agent_id.clone(),
                identity_type: "hardware-rooted".to_string(),
                pcrs: None, // Bridge currently uses base identity, TitanGate handles detailed PCR binding
                metadata: serde_json::Value::Null,
            }
        } else {
            tracing::warn!(
                "AuthorityBridge: No hardware identity attached. Using restricted IdentityData. \
                 Call .with_identity() to enable real attestation."
            );
            IdentityData {
                aid: "mock-aid-01".to_string(),
                identity_type: "unbound".to_string(),
                pcrs: None,
                metadata: serde_json::Value::Null,
            }
        };

        // 4. WitnessData — populated from the live CHORA response.
        let now = chrono::Utc::now().timestamp() as u64;
        let witness = WitnessData {
            chora_node_id: response.authority.capsule_id.clone(),
            receipt_hash: response.signature.clone(),
            timestamp: now,
            metadata: serde_json::Value::Null,
        };

        // 5. Build Pillar Hashes
        let intent_hash = intent.to_jcs_hash()?.to_hex();

        // Helper to hash a JCS segment
        fn hash_seg<T: Serialize>(seg: &T) -> Result<String, String> {
            let jcs = serde_jcs::to_vec(seg).map_err(|e| e.to_string())?;
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(&jcs);
            Ok(hex::encode(hasher.finalize()))
        }

        let authority_hash = hash_seg(&response.authority)?;
        let identity_hash = hash_seg(&identity)?;
        let witness_hash = witness.to_commitment_hash()?.to_hex();

        // 6. Build Composite Capsule
        let mut capsule = Capsule {
            capsule_id: response.authority.capsule_id.clone(),
            intent,
            authority: response.authority,
            identity,
            witness,
            intent_hash,
            authority_hash,
            identity_hash,
            witness_hash,
            capsule_root: String::new(), // Computed below
            crypto: CryptoData {
                algo: "ed25519".to_string(),
                public_key_endpoint: "/public_key".to_string(),
                signature_scope: "capsule_root".to_string(),
                signature_b64: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    hex::decode(&response.signature).unwrap_or_default(),
                ),
            },
            request_commitment: None,
        };

        // 7. Compute Final Capsule Root
        let root = capsule.to_composite_hash()?;
        capsule.capsule_root = root.to_hex();

        Ok(capsule)
    }

    /// Canonicalize a payload for CHORA compliance (RFC 8785)
    pub fn canonicalize<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
        serde_jcs::to_vec(payload).map_err(|e| format!("JCS Canonicalization failed: {}", e))
    }

    /// Verify a Continuation Token using the Authority Client
    pub async fn verify_continuation_token(
        &self,
        token: &vex_core::ContinuationToken,
    ) -> Result<bool, String> {
        self.client.verify_continuation_token(token).await
    }
}
