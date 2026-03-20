use async_trait::async_trait;
use base64::Engine as _;
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
pub trait AuthorityClient: Send + Sync + std::fmt::Debug {
    async fn request_attestation(&self, payload: &[u8]) -> Result<ChoraResponse, String>;
    async fn verify_witness_signature(
        &self,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<bool, String>;
    async fn verify_continuation_token(
        &self,
        token: &vex_core::ContinuationToken,
    ) -> Result<bool, String>;
}

/// A Mock Authority Client for test/dev environments.
/// Generates deterministic signatures based on a test key.
#[derive(Debug)]
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
            reason_code: "OK".into(),
            nonce: 42,
            trace_root: "00".repeat(32), // Mocked trace root
            escalation_id: None,
            binding_status: Some("SHADOW".to_string()),
            continuation_token: Some(vex_core::ContinuationToken {
                payload: vex_core::ContinuationPayload {
                    schema: "chora.continuation.token.v1".to_string(),
                    ledger_event_id: "mock-ledger-id".to_string(),
                    source_capsule_root: "mock-root".to_string(),
                    resolution_event_id: Some("mock-resolve-id".to_string()),
                    nonce: "mock-nonce".to_string(),
                    iat: "2026-03-17T19:06:55Z".to_string(),
                    exp: "2026-03-17T19:16:55Z".to_string(),
                    issuer: "chora-gate-mock".to_string(),
                },
                signature: "mock-sig".to_string(),
            }),
            gate_sensors: serde_json::Value::Null,
            metadata: serde_json::Value::Null,
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

    async fn verify_continuation_token(
        &self,
        _token: &vex_core::ContinuationToken,
    ) -> Result<bool, String> {
        // Mock always returns true for test/dev
        Ok(true)
    }
}

/// A real HTTP Authority Client for production environments.
/// Connects to any CHORA-compatible gate node via HTTP.
/// Configure with CHORA_GATE_URL and CHORA_API_KEY environment variables.
#[derive(Debug)]
pub struct HttpChoraClient {
    client: reqwest::Client,
    base_url: String,
    api_key: String,
}

impl HttpChoraClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(30))
                .build()
                .unwrap_or_default(),
            base_url,
            api_key,
        }
    }

    fn gate_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/gate", base)
    }

    fn public_key_url(&self) -> String {
        let base = self.base_url.trim_end_matches('/');
        format!("{}/public_key", base)
    }
}

#[derive(Debug, Deserialize)]
struct ChoraApiAuthority {
    capsule_id: String,
    outcome: String,
    reason_code: String,
    #[serde(default)]
    nonce: Option<u64>,
    #[serde(default)]
    trace_root: Option<String>,
    /// New Phase 2 fields
    #[serde(default)]
    escalation_id: Option<String>,
    #[serde(default)]
    binding_status: Option<String>,
    #[serde(default, alias = "signed_token")]
    pub continuation_token: Option<vex_core::ContinuationToken>,
}

#[derive(Debug, Deserialize)]
struct ChoraApiResponse {
    #[serde(alias = "signed_payload")]
    authority: Option<ChoraApiAuthority>,
    #[serde(default)]
    capsule_id: Option<String>,
    #[serde(default)]
    outcome: Option<String>,
    #[serde(default)]
    reason_code: Option<String>,
    #[serde(default)]
    signature: Option<String>,
    #[serde(default)]
    witness_receipt: Option<String>,
    /// New Phase 2 fields (flat)
    #[serde(default)]
    escalation_id: Option<String>,
    #[serde(default)]
    binding_status: Option<String>,
    #[serde(default, alias = "signed_token")]
    pub continuation_token: Option<vex_core::ContinuationToken>,
}

#[async_trait]
impl AuthorityClient for HttpChoraClient {
    async fn request_attestation(&self, payload: &[u8]) -> Result<ChoraResponse, String> {
        use sha2::{Digest, Sha256};

        // Derive a SHA-256 based capsule_id from the payload
        let mut hasher = Sha256::new();
        hasher.update(payload);
        let hash = hasher.finalize();
        let payload_hash = hex::encode(hash);

        // POST to /gate with confidence and payload (authority handshake)
        let body = serde_json::json!({
            "confidence": 0.95,
            "payload": base64::engine::general_purpose::STANDARD.encode(payload),
            "payload_hash": payload_hash,
        });

        let resp = self
            .client
            .post(self.gate_url())
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|e| format!("CHORA HTTP request failed: {}", e))?;

        let status = resp.status();
        let text = resp
            .text()
            .await
            .unwrap_or_else(|_| "empty response".to_string());

        if !status.is_success() {
            return Err(format!("CHORA gate returned status {}: {}", status, text));
        }

        let api_resp: ChoraApiResponse = serde_json::from_str(&text)
            .map_err(|e| format!("CHORA response parse failed: {} (raw: {})", e, text))?;

        // Support both response shapes: nested signed_payload (authority) or flat fields
        let capsule_id = api_resp.authority.as_ref()
            .map(|a| a.capsule_id.clone())
            .or_else(|| api_resp.capsule_id.clone())
            .unwrap_or_else(|| payload_hash.clone());

        let outcome = api_resp.authority.as_ref()
            .map(|a| a.outcome.clone())
            .or_else(|| api_resp.outcome.clone())
            .unwrap_or_else(|| "ALLOW".to_string());

        let reason_code = api_resp.authority.as_ref()
            .map(|a| a.reason_code.clone())
            .or_else(|| api_resp.reason_code.clone())
            .unwrap_or_else(|| "OK".to_string());

        let nonce = api_resp.authority.as_ref()
            .and_then(|a| a.nonce)
            .unwrap_or(0);

        let trace_root = api_resp.authority.as_ref()
            .and_then(|a| a.trace_root.clone())
            .or_else(|| api_resp.witness_receipt.clone())
            .unwrap_or_else(|| payload_hash.clone());

        let escalation_id = api_resp.authority.as_ref()
            .and_then(|a| a.escalation_id.clone())
            .or_else(|| api_resp.escalation_id.clone());

        let binding_status = api_resp.authority.as_ref()
            .and_then(|a| a.binding_status.clone())
            .or_else(|| api_resp.binding_status.clone());

        let continuation_token = api_resp.authority.as_ref()
            .and_then(|a| a.continuation_token.clone())
            .or_else(|| api_resp.continuation_token.clone());

        let authority = AuthorityData {
            capsule_id,
            outcome,
            reason_code,
            nonce,
            trace_root,
            escalation_id,
            binding_status,
            continuation_token,
            gate_sensors: serde_json::Value::Null,
            metadata: serde_json::Value::Null,
        };

        let signature = api_resp
            .signature
            .or(api_resp.witness_receipt)
            .unwrap_or(payload_hash);

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

        // Fetch the CHORA node's Ed25519 public key via GET /public_key
        let resp = self
            .client
            .get(self.public_key_url())
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("CHORA public_key fetch failed: {}", e))?;

        let text = resp
            .text()
            .await
            .map_err(|e| format!("CHORA public_key read failed: {}", e))?;
        
        // Live endpoint returns a JSON string, e.g. "e349f464..."
        let hex_key: String = serde_json::from_str(&text)
            .unwrap_or_else(|_| text.trim_matches('"').to_string());

        let raw_key_vec = hex::decode(&hex_key)
            .map_err(|e| format!("Public key hex decode failed: {}", e))?;
        
        let raw_key: [u8; 32] = raw_key_vec.try_into()
            .map_err(|_| "Invalid Ed25519 public key length".to_string())?;

        let verifying_key = VerifyingKey::from_bytes(&raw_key).map_err(|e| e.to_string())?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| "Signature must be 64 bytes".to_string())?;
        let sig = Signature::from_bytes(&sig_bytes);

        Ok(verifying_key.verify(payload, &sig).is_ok())
    }

    async fn verify_continuation_token(
        &self,
        token: &vex_core::ContinuationToken,
    ) -> Result<bool, String> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        // 1. Fetch Key (reuse public_key logic - ideally cached in bridge, but client can call it)
        let resp = self
            .client
            .get(self.public_key_url())
            .header("x-api-key", &self.api_key)
            .send()
            .await
            .map_err(|e| format!("CHORA public_key fetch failed: {}", e))?;

        let text = resp.text().await.map_err(|e| e.to_string())?;
        let hex_key: String = serde_json::from_str(&text)
            .unwrap_or_else(|_| text.trim_matches('"').to_string());

        let raw_key_vec = hex::decode(&hex_key).map_err(|e| e.to_string())?;
        let raw_key: [u8; 32] = raw_key_vec.try_into().map_err(|_| "Invalid Key".to_string())?;

        let verifying_key = VerifyingKey::from_bytes(&raw_key).map_err(|e| e.to_string())?;

        // 2. Decode Signature
        let sig_bytes = hex::decode(&token.signature).map_err(|e| e.to_string())?;
        let sig = Signature::from_bytes(sig_bytes.as_slice().try_into().map_err(|_| "Invalid Sig")?);
 
        // 3. Lifecycle Validation
        token.payload.validate_lifecycle(chrono::Utc::now())
            .map_err(|e| format!("Lifecycle check failed: {}", e))?;

        // 4. Verify Signature with George's v3 Specs:
        // - RFC 8785 (JCS) Canonical JSON
        // - Signed as raw UTF-8 bytes (NOT hashed)
        let jcs_bytes = serde_jcs::to_vec(&token.payload).map_err(|e| e.to_string())?;
        
        if verifying_key.verify(&jcs_bytes, &sig).is_ok() {
            return Ok(true);
        }

        tracing::error!(
            jcs_hex = %hex::encode(&jcs_bytes),
            jcs_utf8 = %String::from_utf8_lossy(&jcs_bytes),
            "CHORA: Signature verification failed natively using V3 RFC 8785."
        );
        
        Ok(false)
    }
}

/// Factory: creates a real HttpChoraClient.
/// Used by vex-server to avoid importing the concrete type directly.
pub fn make_authority_client(url: String, api_key: String) -> std::sync::Arc<dyn AuthorityClient> {
    std::sync::Arc::new(HttpChoraClient::new(url, api_key))
}

/// Factory: creates a MockChoraClient for local dev / CI.
pub fn make_mock_client() -> std::sync::Arc<dyn AuthorityClient> {
    std::sync::Arc::new(MockChoraClient)
}

#[cfg(test)]
mod tests {
    use ed25519_dalek::Verifier;
    #[tokio::test]
    async fn test_george_token_verification() {
        use base64::Engine as _;
        let token_json = r#"{
            "payload": {
                "schema": "chora.continuation.token.v3",
                "issuer": "chora-gate-v0.3",
                "iat": "2026-03-19T16:57:27.542164+00:00",
                "exp": "2026-03-19T17:07:27.542164+00:00",
                "ledger_event_id": "763331c3-3b82-485d-b449-0f6f033a5203",
                "resolution_event_id": "ems-resolve-763331c3-3b82-485d-b449-0f6f033a5203",
                "source_capsule_root": "ef7e9de0b541489e249ce4f7c6f49c078d5537be512592c52215e7441222037d",
                "nonce": "nonce-750001"
            },
            "signature": "6e6e140025ce60e471a903d787db85c65b5474d743c6e6cda2901b66e63b52e047a4a781815d6df6e9714f8c383a79d8d08661aba774f949f0c242a5884dd201"
        }"#;

        let token: vex_core::ContinuationToken = serde_json::from_str(token_json).unwrap();
        
        // Key inside public_key_token.pem (SPKI decoded)
        // MCowBQYDK2VwAyEAFlfgFLXnzyyB8exqQ+DUDH+tWGX9zaUIHwhC1glFsr4=
        // The last 32 bytes are the raw Ed25519 public key.
        let pub_key_b64 = "FlfgFLXnzyyB8exqQ+DUDH+tWGX9zaUIHwhC1glFsr4=";
        let raw_key_bytes = base64::engine::general_purpose::STANDARD.decode(pub_key_b64).unwrap();
        
        let verifying_key = ed25519_dalek::VerifyingKey::try_from(raw_key_bytes.as_slice()).unwrap();
        let sig_bytes = hex::decode(&token.signature).unwrap();
        let sig = ed25519_dalek::Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());
        
        // In v3, the payload is signed after canonicalization using RFC 8785 (JCS)
        let jcs_bytes = serde_jcs::to_vec(&token.payload).unwrap();
        
        println!("JCS payload: {}", String::from_utf8_lossy(&jcs_bytes));

        assert!(verifying_key.verify(&jcs_bytes, &sig).is_ok(), "Signature verification failed for Canonical JCS encoded Token V3");
    }
}
