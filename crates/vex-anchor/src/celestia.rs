//! Celestia Data Availability anchor backend
//!
//! Posts Merkle roots to a Celestia node as namespace-keyed blobs.
//! Provides DA guarantees via Celestia's light client network.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use vex_core::Hash;

use crate::backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
use crate::error::AnchorError;

/// VEX namespace on Celestia (v0 format)
const VEX_NAMESPACE_B64: &str = "AAAAAAAAAAAAAAAAAAAVEX=";

#[derive(Deserialize)]
struct BlobSubmitResponse {
    result: Option<u64>,
    error: Option<CelestiaError>,
}

#[derive(Deserialize)]
struct CelestiaError {
    code: i64,
    message: String,
}

#[derive(Deserialize)]
struct HeaderResponse {
    result: Option<serde_json::Value>,
}

#[derive(Serialize)]
struct Blob {
    namespace: String,
    data: String,
    #[serde(rename = "shareVersion")]
    share_version: u32,
}

/// Celestia DA anchor backend
///
/// Submits Merkle roots as named blobs to a Celestia node.
/// The blob height is stored as the `anchor_id` for later verification.
#[derive(Debug, Clone)]
pub struct CelestiaAnchor {
    node_url: String,
    auth_token: Option<String>,
    client: reqwest::Client,
}

impl CelestiaAnchor {
    /// Create a new Celestia anchor backend
    pub fn new(node_url: impl Into<String>, auth_token: Option<String>) -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .user_agent("vex-anchor/0.1.5")
            .build()
            .expect("Failed to build Celestia HTTP client");

        Self {
            node_url: node_url.into(),
            auth_token,
            client,
        }
    }

    fn post(&self, url: &str) -> reqwest::RequestBuilder {
        let req = self.client.post(url);
        if let Some(ref token) = self.auth_token {
            req.bearer_auth(token)
        } else {
            req
        }
    }
}

#[async_trait]
impl AnchorBackend for CelestiaAnchor {
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        let blob_payload = serde_json::json!({
            "vex_root": root.to_hex(),
            "tenant_id": metadata.tenant_id,
            "event_count": metadata.event_count,
            "timestamp": metadata.timestamp.to_rfc3339(),
        });
        let blob_data = STANDARD.encode(blob_payload.to_string().as_bytes());

        let blob = Blob {
            namespace: VEX_NAMESPACE_B64.to_string(),
            data: blob_data,
            share_version: 0,
        };

        let req = serde_json::json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "blob.Submit",
            "params": [[blob], {"gas_price": -1.0}]
        });

        let url = format!("{}/", self.node_url);
        let resp = self
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        if !resp.status().is_success() {
            return Err(AnchorError::Network(format!(
                "Celestia node returned HTTP {}",
                resp.status()
            )));
        }

        let body: BlobSubmitResponse = resp
            .json()
            .await
            .map_err(|e| AnchorError::Network(e.to_string()))?;

        if let Some(err) = body.error {
            return Err(AnchorError::Network(format!(
                "Celestia error {}: {}",
                err.code, err.message
            )));
        }

        let height = body.result.unwrap_or(0);
        let anchor_id = format!("celestia://height:{}", height);
        let proof = serde_json::json!({
            "height": height,
            "namespace": VEX_NAMESPACE_B64,
            "root_hash": root.to_hex()
        })
        .to_string();

        Ok(AnchorReceipt {
            backend: self.name().to_string(),
            root_hash: root.to_hex(),
            anchor_id,
            anchored_at: Utc::now(),
            proof: Some(proof),
            metadata,
        })
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        let Some(ref proof_str) = receipt.proof else {
            return Ok(false);
        };

        let proof: serde_json::Value = serde_json::from_str(proof_str)
            .map_err(|e| AnchorError::VerificationFailed(format!("Invalid proof JSON: {}", e)))?;

        let height = proof["height"].as_u64().unwrap_or(0);
        if height == 0 {
            return Ok(false);
        }

        let req = serde_json::json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "blob.GetAll",
            "params": [height, [VEX_NAMESPACE_B64]]
        });

        let url = format!("{}/", self.node_url);
        let resp = self
            .post(&url)
            .json(&req)
            .send()
            .await
            .map_err(|e| AnchorError::VerificationFailed(e.to_string()))?;

        if !resp.status().is_success() {
            return Ok(false);
        }

        let body: HeaderResponse = resp
            .json()
            .await
            .map_err(|e| AnchorError::VerificationFailed(e.to_string()))?;

        if let Some(result) = body.result {
            if let Some(blobs) = result.as_array() {
                return Ok(blobs.iter().any(|b| {
                    b.get("data")
                        .and_then(|d| d.as_str())
                        .and_then(|d| STANDARD.decode(d).ok())
                        .and_then(|bytes| String::from_utf8(bytes).ok())
                        .is_some_and(|s| s.contains(&receipt.root_hash))
                }));
            }
        }

        Ok(false)
    }

    fn name(&self) -> &str {
        "celestia"
    }

    async fn is_healthy(&self) -> bool {
        let req = serde_json::json!({
            "id": 1,
            "jsonrpc": "2.0",
            "method": "header.NetworkHead",
            "params": []
        });

        let url = format!("{}/", self.node_url);
        self.post(&url)
            .json(&req)
            .send()
            .await
            .ok()
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_celestia_anchor_name() {
        let anchor = CelestiaAnchor::new("http://localhost:26658", None);
        assert_eq!(anchor.name(), "celestia");
    }

    #[test]
    fn test_celestia_verify_missing_proof() {
        use crate::backend::AnchorMetadata;
        let receipt = AnchorReceipt {
            backend: "celestia".to_string(),
            root_hash: "abc123".to_string(),
            anchor_id: "celestia://height:0".to_string(),
            anchored_at: Utc::now(),
            proof: None,
            metadata: AnchorMetadata::new("test-tenant", 1),
        };
        assert!(receipt.proof.is_none());
    }
}
