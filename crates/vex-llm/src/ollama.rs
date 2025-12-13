//! Ollama LLM provider for local inference

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Ollama API request format
#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    system: Option<String>,
    stream: bool,
    options: OllamaOptions,
}

#[derive(Debug, Serialize)]
struct OllamaOptions {
    temperature: f32,
    num_predict: u32,
}

/// Ollama API response format
#[derive(Debug, Deserialize)]
struct OllamaApiResponse {
    response: String,
    model: String,
    #[serde(default)]
    eval_count: Option<u32>,
}

/// Ollama provider for local LLM inference
#[derive(Debug)]
pub struct OllamaProvider {
    /// Base URL for Ollama API
    base_url: String,
    /// Model to use (e.g., "llama2", "mistral", "codellama")
    model: String,
    /// HTTP client
    client: reqwest::Client,
}

impl OllamaProvider {
    /// Create a new Ollama provider with default settings
    pub fn new(model: &str) -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }

    /// Create with custom base URL
    pub fn with_url(base_url: &str, model: &str) -> Self {
        Self {
            base_url: base_url.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
        }
    }
}

#[async_trait]
impl LlmProvider for OllamaProvider {
    fn name(&self) -> &str {
        "ollama"
    }

    async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client.get(&url).send().await.is_ok()
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();
        let url = format!("{}/api/generate", self.base_url);

        let ollama_request = OllamaRequest {
            model: self.model.clone(),
            prompt: request.prompt,
            system: Some(request.system),
            stream: false,
            options: OllamaOptions {
                temperature: request.temperature,
                num_predict: request.max_tokens,
            },
        };

        let response = self
            .client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            return Err(LlmError::RequestFailed(format!(
                "Status: {}",
                response.status()
            )));
        }

        let api_response: OllamaApiResponse = response
            .json()
            .await
            .map_err(|e| LlmError::InvalidResponse(e.to_string()))?;

        Ok(LlmResponse {
            content: api_response.response,
            model: api_response.model,
            tokens_used: api_response.eval_count,
            latency_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires Ollama running locally
    async fn test_ollama_available() {
        let provider = OllamaProvider::new("llama2");
        // This test is ignored by default since it requires Ollama
        if provider.is_available().await {
            let response = provider.ask("Say hello in one word").await.unwrap();
            assert!(!response.is_empty());
        }
    }
}
