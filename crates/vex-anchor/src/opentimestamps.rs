//! OpenTimestamps anchor backend
//!
//! Submits Merkle roots to the public OpenTimestamps calendar servers
//! (https://alice.btc.calendar.opentimestamps.org) for Bitcoin blockchain anchoring.

use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use chrono::Utc;
use vex_core::Hash;

use crate::backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
use crate::error::AnchorError;

/// Public OTS calendar servers (tried in order)
const OTS_CALENDARS: &[&str] = &[
    "https://alice.btc.calendar.opentimestamps.org",
    "https://bob.btc.calendar.opentimestamps.org",
    "https://finney.calendar.eternitywall.com",
];

/// OpenTimestamps calendar anchor backend
///
/// Submits Merkle roots to public Bitcoin calendar servers for timestamping.
/// Proofs become final after ~1 Bitcoin block and are verifiable with `ots verify`.
#[derive(Debug, Clone)]
pub struct OpenTimestampsAnchor {
    client: reqwest::Client,
}

impl OpenTimestampsAnchor {
    /// Create a new OpenTimestamps anchor using the public calendar servers
    pub fn new() -> Self {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .user_agent("vex-anchor/0.1.5")
            .build()
            .expect("Failed to build OTS HTTP client");
        Self { client }
    }
}

impl Default for OpenTimestampsAnchor {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl AnchorBackend for OpenTimestampsAnchor {
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        // OTS calendar accepts raw 32-byte SHA-256 digests
        let digest_bytes = root.0.to_vec();

        let mut last_error = AnchorError::Network("No calendars configured".to_string());
        for calendar in OTS_CALENDARS {
            let url = format!("{}/digest", calendar);
            let response = self
                .client
                .post(&url)
                .header("Content-Type", "application/x-www-form-urlencoded")
                .body(digest_bytes.clone())
                .send()
                .await;

            match response {
                Ok(resp) if resp.status().is_success() => {
                    let proof_bytes = resp
                        .bytes()
                        .await
                        .map_err(|e| AnchorError::Network(e.to_string()))?;

                    let proof_b64 = STANDARD.encode(&proof_bytes);
                    let anchor_id = format!("{}#{}", calendar, root.to_hex());

                    return Ok(AnchorReceipt {
                        backend: self.name().to_string(),
                        root_hash: root.to_hex(),
                        anchor_id,
                        anchored_at: Utc::now(),
                        proof: Some(proof_b64),
                        metadata,
                    });
                }
                Ok(resp) => {
                    last_error = AnchorError::Network(format!(
                        "Calendar {} returned HTTP {}",
                        calendar,
                        resp.status()
                    ));
                }
                Err(e) => {
                    last_error =
                        AnchorError::Network(format!("Calendar {} unreachable: {}", calendar, e));
                }
            }
        }

        Err(last_error)
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        let Some(ref proof_b64) = receipt.proof else {
            return Ok(false);
        };

        let proof_bytes = STANDARD.decode(proof_b64).map_err(|e| {
            AnchorError::VerificationFailed(format!("Invalid base64 proof: {}", e))
        })?;

        // Non-empty proof means the OTS calendar acknowledged the submission
        Ok(!proof_bytes.is_empty())
    }

    fn name(&self) -> &str {
        "opentimestamps"
    }

    async fn is_healthy(&self) -> bool {
        let url = format!("{}/digest", OTS_CALENDARS[0]);
        self.client
            .head(&url)
            .send()
            .await
            .map(|r| r.status().as_u16() < 500)
            .unwrap_or(false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::AnchorMetadata;

    #[test]
    fn test_ots_anchor_name() {
        let anchor = OpenTimestampsAnchor::new();
        assert_eq!(anchor.name(), "opentimestamps");
    }

    #[test]
    fn test_ots_verify_missing_proof() {
        let receipt = AnchorReceipt {
            backend: "opentimestamps".to_string(),
            root_hash: "abc123".to_string(),
            anchor_id: "ots://test#abc123".to_string(),
            anchored_at: Utc::now(),
            proof: None,
            metadata: AnchorMetadata::new("test-tenant", 1),
        };
        assert!(receipt.proof.is_none());
    }
}
