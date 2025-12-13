//! LLM Provider trait and common types

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use thiserror::Error;

/// Errors from LLM providers
#[derive(Debug, Error)]
pub enum LlmError {
    #[error("Connection failed: {0}")]
    ConnectionFailed(String),
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("Invalid response: {0}")]
    InvalidResponse(String),
    #[error("Rate limited")]
    RateLimited,
    #[error("Provider not available")]
    NotAvailable,
}

/// A request to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// System prompt (role/persona)
    pub system: String,
    /// User message
    pub prompt: String,
    /// Temperature (0.0 = deterministic, 1.0 = creative)
    pub temperature: f32,
    /// Maximum tokens to generate
    pub max_tokens: u32,
}

impl LlmRequest {
    /// Create a simple request with default settings
    pub fn simple(prompt: &str) -> Self {
        Self {
            system: "You are a helpful assistant.".to_string(),
            prompt: prompt.to_string(),
            temperature: 0.7,
            max_tokens: 1024,
        }
    }

    /// Create a request with a specific role
    pub fn with_role(system: &str, prompt: &str) -> Self {
        Self {
            system: system.to_string(),
            prompt: prompt.to_string(),
            temperature: 0.7,
            max_tokens: 1024,
        }
    }
}

/// Response from an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    /// The generated text
    pub content: String,
    /// Model used
    pub model: String,
    /// Tokens used (if available)
    pub tokens_used: Option<u32>,
    /// Time taken in milliseconds
    pub latency_ms: u64,
}

/// Trait for LLM providers
#[async_trait]
pub trait LlmProvider: Send + Sync + std::fmt::Debug {
    /// Get the provider name
    fn name(&self) -> &str;

    /// Check if the provider is available
    async fn is_available(&self) -> bool;

    /// Generate a completion
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;

    /// Generate with a simple prompt (convenience method)
    async fn ask(&self, prompt: &str) -> Result<String, LlmError> {
        let response = self.complete(LlmRequest::simple(prompt)).await?;
        Ok(response.content)
    }
}
