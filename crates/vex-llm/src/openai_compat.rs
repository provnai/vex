//! OpenAI-compatible provider — shared implementation for OpenAI, DeepSeek, Mistral, Groq, etc.
//!
//! All providers that speak the OpenAI `/v1/chat/completions` protocol can use this
//! struct directly or wrap it with provider-specific convenience constructors.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Chat message in the OpenAI-compatible format
#[derive(Debug, Serialize)]
pub struct ChatMessage {
    pub role: String,
    pub content: String,
}

/// Request body for OpenAI-compatible chat completion APIs
#[derive(Debug, Serialize)]
pub struct ChatCompletionRequest {
    pub model: String,
    pub messages: Vec<ChatMessage>,
    pub temperature: f32,
    pub max_tokens: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presence_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub frequency_penalty: Option<f32>,
}

/// A single choice in a chat completion response
#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: MessageContent,
}

/// Message content within a choice
#[derive(Debug, Deserialize)]
pub struct MessageContent {
    pub content: String,
}

/// Token usage statistics
#[derive(Debug, Deserialize)]
pub struct Usage {
    pub total_tokens: u32,
}

/// Chat completion response from an OpenAI-compatible API
#[derive(Debug, Deserialize)]
pub struct ChatCompletionResponse {
    pub choices: Vec<Choice>,
    pub model: String,
    pub usage: Option<Usage>,
}

/// A provider that speaks the OpenAI `/v1/chat/completions` protocol.
///
/// This single struct replaces duplicated implementations across OpenAI, DeepSeek,
/// and Mistral providers. Provider-specific wrappers supply convenience constructors
/// and provider-specific defaults.
#[derive(Debug, Clone)]
pub struct OpenAICompatibleProvider {
    /// API key for authentication
    pub api_key: String,
    /// Model identifier (e.g., "gpt-4", "deepseek-chat", "mistral-large-latest")
    pub model: String,
    /// HTTP client
    pub client: reqwest::Client,
    /// Base URL for the API (e.g., "https://api.openai.com")
    pub base_url: String,
    /// Default request timeout
    pub default_timeout: std::time::Duration,
    /// Human-readable provider name (e.g., "openai", "deepseek", "mistral")
    pub provider_name: String,
}

impl OpenAICompatibleProvider {
    /// Create a new OpenAI-compatible provider
    pub fn new(
        api_key: impl Into<String>,
        model: impl Into<String>,
        base_url: impl Into<String>,
        provider_name: impl Into<String>,
    ) -> Self {
        let timeout = std::env::var("VEX_LLM_TIMEOUT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(30);

        Self {
            api_key: api_key.into(),
            model: model.into(),
            client: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(timeout))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
            base_url: base_url.into(),
            default_timeout: std::time::Duration::from_secs(timeout),
            provider_name: provider_name.into(),
        }
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }
}

#[async_trait]
impl LlmProvider for OpenAICompatibleProvider {
    fn name(&self) -> &str {
        &self.provider_name
    }

    async fn is_available(&self) -> bool {
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
            ChatMessage {
                role: "system".to_string(),
                content: request.system,
            },
            ChatMessage {
                role: "user".to_string(),
                content: request.prompt,
            },
        ];

        let api_request = ChatCompletionRequest {
            model: self.model.clone(),
            messages,
            temperature: request.temperature,
            max_tokens: request.max_tokens,
            top_p: request.top_p,
            presence_penalty: request.presence_penalty,
            frequency_penalty: request.frequency_penalty,
        };

        let request_timeout = request.timeout.unwrap_or(self.default_timeout);

        let response = tokio::time::timeout(
            request_timeout,
            self.client
                .post(&url)
                .bearer_auth(&self.api_key)
                .json(&api_request)
                .send(),
        )
        .await
        .map_err(|_| LlmError::Timeout(request_timeout.as_millis() as u64))?
        .map_err(|e| LlmError::ConnectionFailed(e.to_string()))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();

            if status == reqwest::StatusCode::TOO_MANY_REQUESTS {
                return Err(LlmError::RateLimited);
            }

            return Err(LlmError::RequestFailed(format!(
                "Status: {}, Body: {}",
                status, body
            )));
        }

        let api_response: ChatCompletionResponse = response
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
