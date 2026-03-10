use crate::error::AttestError;
use provn_sdk::SignedClaim;
use serde::{Deserialize, Serialize};
use vex_anchor::AnchorMetadata;

#[derive(Serialize)]
pub struct VerifyRequest {
    pub claim: String,
    pub anchor_to_blockchain: bool,
    pub anchor_mode: Option<String>,
    pub anchor_payload: Option<serde_json::Value>,
}

#[derive(Deserialize)]
pub struct VerifyResponse {
    pub proof_hash: String,
    pub anchor_id: Option<String>,
    pub solscan_url: Option<String>,
    pub status: String,
}

pub struct ProvnCloudClient {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl ProvnCloudClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// Upload a raw SignedClaim from provn-sdk
    pub async fn upload_event_raw(
        &self,
        signed_claim: SignedClaim,
        metadata: Option<AnchorMetadata>,
    ) -> Result<VerifyResponse, AttestError> {
        let url = format!("{}/api/verify", self.base_url);

        let request = VerifyRequest {
            claim: signed_claim.claim.data.clone(),
            anchor_to_blockchain: true,
            anchor_mode: Some("standard".to_string()),
            anchor_payload: Some(serde_json::json!({
                "signed_claim": signed_claim,
                "metadata": metadata
            })),
        };

        let response = self
            .client
            .post(&url)
            .header("x-api-key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            // Manual error for non-success status
            return Err(AttestError::Unknown(format!(
                "API returned status {}",
                response.status()
            )));
        }

        Ok(response.json::<VerifyResponse>().await?)
    }

    pub async fn get_status(&self, anchor_id: &str) -> Result<String, AttestError> {
        let url = format!("{}/api/anchor/status/{}", self.base_url, anchor_id);

        let response = self
            .client
            .get(&url)
            .header("x-api-key", &self.api_key)
            .send()
            .await?
            .json::<serde_json::Value>()
            .await?;

        Ok(response["status"].as_str().unwrap_or("unknown").to_string())
    }

    pub async fn is_healthy(&self) -> bool {
        let url = format!("{}/health", self.base_url);
        self.client
            .get(&url)
            .send()
            .await
            .map(|r| r.status().is_success())
            .unwrap_or(false)
    }
}
