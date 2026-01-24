//! Mistral AI LLM provider (OpenAI-compatible API)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Mistral API request format (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct MistralRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f32,
    max_tokens: u32,
}

#[derive(Debug, Serialize)]
struct Message {
    role: String,
    content: String,
}

/// Mistral API response format
#[derive(Debug, Deserialize)]
struct MistralResponse {
    choices: Vec<Choice>,
    model: String,
    usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: MessageContent,
}

#[derive(Debug, Deserialize)]
struct MessageContent {
    content: String,
}

#[derive(Debug, Deserialize)]
struct Usage {
    total_tokens: u32,
}

/// Mistral AI provider for inference
#[derive(Debug)]
pub struct MistralProvider {
    /// API key
    api_key: String,
    /// Model to use (e.g., "mistral-large-latest", "mistral-small-latest", "codestral-latest")
    model: String,
    /// HTTP client
    client: reqwest::Client,
    /// Base URL
    base_url: String,
}

impl MistralProvider {
    /// Create a new Mistral provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
            base_url: "https://api.mistral.ai".to_string(),
        }
    }

    /// Create with Mistral Large 3 (state-of-the-art, open-weight, general-purpose multimodal model)
    pub fn large(api_key: &str) -> Self {
        Self::new(api_key, "mistral-large-latest")
    }

    /// Create with Mistral Medium 3.1 (frontier-class multimodal model)
    pub fn medium(api_key: &str) -> Self {
        Self::new(api_key, "mistral-medium-latest")
    }

    /// Create with Mistral Small 3.2 (fast and efficient)
    pub fn small(api_key: &str) -> Self {
        Self::new(api_key, "mistral-small-latest")
    }

    /// Create with Codestral (cutting-edge code generation model)
    pub fn codestral(api_key: &str) -> Self {
        Self::new(api_key, "codestral-latest")
    }

    /// Create with Devstral (excels at software engineering use cases)
    pub fn devstral(api_key: &str) -> Self {
        Self::new(api_key, "devstral-small-latest")
    }

    /// Create with Ministral 8B (lightweight model with best-in-class text and vision)
    pub fn ministral_8b(api_key: &str) -> Self {
        Self::new(api_key, "ministral-8b-latest")
    }

    /// Create with Ministral 3B (tiny and efficient model)
    pub fn ministral_3b(api_key: &str) -> Self {
        Self::new(api_key, "ministral-3b-latest")
    }

    /// Create with Pixtral Large (frontier-class multimodal vision model)
    pub fn pixtral(api_key: &str) -> Self {
        Self::new(api_key, "pixtral-large-latest")
    }

    /// Create with Mistral Nemo 12B (multilingual open source model)
    pub fn nemo(api_key: &str) -> Self {
        Self::new(api_key, "open-mistral-nemo")
    }

    /// Set a custom base URL (useful for self-hosted or proxy setups)
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.base_url = base_url.to_string();
        self
    }
}

#[async_trait]
impl LlmProvider for MistralProvider {
    fn name(&self) -> &str {
        "mistral"
    }

    async fn is_available(&self) -> bool {
        // Simple check - try to reach the API
        self.client
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
            .send()
            .await
            .is_ok()
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();
        let url = format!("{}/v1/chat/completions", self.base_url);

        let messages = vec![
            Message {
                role: "system".to_string(),
                content: request.system,
            },
            Message {
                role: "user".to_string(),
                content: request.prompt,
            },
        ];

        let mistral_request = MistralRequest {
            model: self.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&mistral_request)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            // Handle rate limiting specifically
            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LlmError::RateLimited);
            }

            return Err(LlmError::RequestFailed(format!(
                "Status: {}, Body: {}",
                status, body
            )));
        }

        let api_response: MistralResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        let content = api_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .unwrap_or_default();

        Ok(LlmResponse {
            content,
            model: api_response.model,
            tokens_used: api_response.usage.map(|u| u.total_tokens),
            latency_ms: start.elapsed().as_millis() as u64,
            trace_root: None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_mistral() {
        let api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
        let provider = MistralProvider::small(&api_key);

        if provider.is_available().await {
            let response = provider.ask("Say hello in one word").await.unwrap();
            assert!(!response.is_empty());
            println!("Mistral response: {}", response);
        }
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_mistral_large() {
        let api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
        let provider = MistralProvider::large(&api_key);

        if provider.is_available().await {
            let response = provider.ask("What is 2+2?").await.unwrap();
            assert!(!response.is_empty());
            println!("Mistral Large response: {}", response);
        }
    }

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_codestral() {
        let api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
        let provider = MistralProvider::codestral(&api_key);

        if provider.is_available().await {
            let response = provider
                .ask("Write a simple hello world function in Rust")
                .await
                .unwrap();
            assert!(!response.is_empty());
            println!("Codestral response: {}", response);
        }
    }
}
