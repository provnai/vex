//! OpenAI LLM provider

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// OpenAI API request format
#[derive(Debug, Serialize)]
struct OpenAIRequest {
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

/// OpenAI API response format
#[derive(Debug, Deserialize)]
struct OpenAIResponse {
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

/// OpenAI provider
#[derive(Debug)]
pub struct OpenAIProvider {
    /// API key
    api_key: String,
    /// Model to use (e.g., "gpt-4", "gpt-3.5-turbo")
    model: String,
    /// HTTP client
    client: reqwest::Client,
    /// Base URL
    base_url: String,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            api_key: api_key.to_string(),
            model: model.to_string(),
            client: reqwest::Client::new(),
            base_url: "https://api.openai.com".to_string(),
        }
    }

    /// Create with GPT-4
    pub fn gpt4(api_key: &str) -> Self {
        Self::new(api_key, "gpt-4")
    }

    /// Create with GPT-4 Turbo
    pub fn gpt4_turbo(api_key: &str) -> Self {
        Self::new(api_key, "gpt-4-turbo-preview")
    }

    /// Create with GPT-3.5 Turbo
    pub fn gpt35(api_key: &str) -> Self {
        Self::new(api_key, "gpt-3.5-turbo")
    }
}

#[async_trait]
impl LlmProvider for OpenAIProvider {
    fn name(&self) -> &str {
        "openai"
    }

    async fn is_available(&self) -> bool {
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

        let openai_request = OpenAIRequest {
            model: self.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
        };

        let response = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&openai_request)
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

        let api_response: OpenAIResponse = response
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
