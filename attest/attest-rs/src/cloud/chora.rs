use crate::error::AttestError;
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct HandshakeRequest {
    pub confidence: f64,
}

#[derive(Deserialize, Debug)]
pub struct VerifyResponse {
    // Basic fields matching what we might expect from George's endpoint
    pub receipt_present: Option<bool>,
    // we can add other verification fields we need later
}

pub struct ChoraClient {
    pub base_url: String,
    pub api_key: String,
    client: reqwest::Client,
}

impl ChoraClient {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            base_url,
            api_key,
            client: reqwest::Client::new(),
        }
    }

    /// POST /gate - Performs the handshake and requests a signed capsule
    pub async fn handshake(&self, confidence: f64) -> Result<serde_json::Value, AttestError> {
        let url = format!("{}/gate", self.base_url);
        let request = HandshakeRequest { confidence };

        let response = self
            .client
            .post(&url)
            .header("X-API-Key", &self.api_key)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AttestError::Unknown(format!(
                "CHORA /gate API returned status {} - {:?}",
                response.status(),
                response.text().await.unwrap_or_default()
            )));
        }

        // Return the raw JSON value (which is a signed .capsule)
        Ok(response.json::<serde_json::Value>().await?)
    }

    /// GET /capsules/{capsule_id}/verify
    pub async fn verify_capsule(&self, capsule_id: &str) -> Result<VerifyResponse, AttestError> {
        let url = format!("{}/capsules/{}/verify", self.base_url, capsule_id);

        let response = self
            .client
            .get(&url)
            .header("X-API-Key", &self.api_key) // might not be needed for verify, but safe to include
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(AttestError::Unknown(format!(
                "CHORA verify API returned status {}",
                response.status()
            )));
        }

        Ok(response.json::<VerifyResponse>().await?)
    }

    /// GET /health
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
