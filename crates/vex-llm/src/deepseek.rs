//! DeepSeek LLM provider (OpenAI-compatible API)

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// DeepSeek API request format (OpenAI-compatible)
#[derive(Debug, Serialize)]
struct DeepSeekRequest {
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

/// DeepSeek API response format
#[derive(Debug, Deserialize)]
struct DeepSeekResponse {
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

/// DeepSeek provider for inference
#[derive(Debug)]
pub struct DeepSeekProvider {
    /// API key
    api_key: String,
    /// Model to use (e.g., "deepseek-chat", "deepseek-coder")
    model: String,
    /// HTTP client
    client: reqwest::Client,
    /// Base URL
    base_url: String,
}

impl DeepSeekProvider {
    /// Create a new DeepSeek provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
            base_url: "https://api.deepseek.com".to_string(),
        }
    }

    /// Create with default chat model
    pub fn chat(api_key: &str) -> Self {
        Self::new(api_key, "deepseek-chat")
    }

    /// Create with coder model
    pub fn coder(api_key: &str) -> Self {
        Self::new(api_key, "deepseek-coder")
    }
}

#[async_trait]
impl LlmProvider for DeepSeekProvider {
    fn name(&self) -> &str {
        "deepseek"
    }

    async fn is_available(&self) -> bool {
        // Simple check - try to reach the API
        self.client
            .get(&format!("{}/v1/models", self.base_url))
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

        let deepseek_request = DeepSeekRequest {
            model: self.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&deepseek_request)
            .send()
            .await
            .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(LlmError::RequestFailed(format!(
                "Status: {}, Body: {}",
                status, body
            )));
        }

        let api_response: DeepSeekResponse = response
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
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires valid API key
    async fn test_deepseek() {
        let api_key = std::env::var("DEEPSEEK_API_KEY").expect("DEEPSEEK_API_KEY not set");
        let provider = DeepSeekProvider::chat(&api_key);

        if provider.is_available().await {
            let response = provider.ask("Say hello in one word").await.unwrap();
            assert!(!response.is_empty());
            println!("DeepSeek response: {}", response);
        }
    }
}
