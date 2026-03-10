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
pub trait AuthorityClient: Send + Sync {
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
            reason_code: "OK".into(),
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

/// A real HTTP Authority Client for production environments.
/// Connects to any CHORA-compatible gate node via HTTP.
/// Configure with CHORA_GATE_URL and CHORA_API_KEY environment variables.
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

        // POST to /gate with JSON payload (authority handshake)
        let body = serde_json::json!({
            "payload_hash": payload_hash,
            "payload_size": payload.len(),
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

        // Support both response shapes: nested signed_payload or flat fields
        let (capsule_id, outcome, reason_code, nonce, trace_root_hex) =
            if let Some(auth) = api_resp.authority {
                (
                    auth.capsule_id,
                    auth.outcome,
                    auth.reason_code,
                    auth.nonce.unwrap_or(0),
                    auth.trace_root,
                )
            } else {
                (
                    api_resp.capsule_id.unwrap_or_else(|| payload_hash.clone()),
                    api_resp.outcome.unwrap_or_else(|| "ALLOW".to_string()),
                    api_resp.reason_code.unwrap_or_else(|| "OK".to_string()),
                    0u64,
                    None,
                )
            };

        // Convert trace_root hex string to [u8; 32], default to zeros
        let mut trace_root = [0u8; 32];
        if let Some(hex_str) = trace_root_hex {
            if let Ok(bytes) = hex::decode(&hex_str) {
                if bytes.len() == 32 {
                    trace_root.copy_from_slice(&bytes);
                }
            }
        }

        let authority = AuthorityData {
            capsule_id,
            outcome,
            reason_code,
            nonce,
            trace_root,
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

        let pem = resp
            .text()
            .await
            .map_err(|e| format!("CHORA public_key read failed: {}", e))?;

        // Parse PEM: strip header/footer, decode base64
        let b64 = pem
            .lines()
            .filter(|l| !l.starts_with("-----"))
            .collect::<Vec<_>>()
            .join("");

        let key_bytes =
            base64::Engine::decode(&base64::engine::general_purpose::STANDARD, b64.trim())
                .map_err(|e| format!("Public key base64 decode failed: {}", e))?;

        // Ed25519 SubjectPublicKeyInfo DER is 44 bytes; raw key is the last 32
        let raw_key: [u8; 32] = key_bytes[key_bytes.len().saturating_sub(32)..]
            .try_into()
            .map_err(|_| "Invalid Ed25519 public key length".to_string())?;

        let verifying_key = VerifyingKey::from_bytes(&raw_key).map_err(|e| e.to_string())?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| "Signature must be 64 bytes".to_string())?;
        let sig = Signature::from_bytes(&sig_bytes);

        Ok(verifying_key.verify(payload, &sig).is_ok())
    }
}

/// Factory: creates a real HttpChoraClient.
/// Used by vex-server to avoid importing the concrete type directly.
pub fn make_authority_client(url: String, api_key: String) -> Box<dyn AuthorityClient> {
    Box::new(HttpChoraClient::new(url, api_key))
}

/// Factory: creates a MockChoraClient for local dev / CI.
pub fn make_mock_client() -> Box<dyn AuthorityClient> {
    Box::new(MockChoraClient)
}
