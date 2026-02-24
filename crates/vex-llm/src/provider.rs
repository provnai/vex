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
    #[error("Input too large: {0} bytes exceeds maximum {1} bytes")]
    InputTooLarge(usize, usize),
}

/// Maximum allowed prompt size in bytes (100KB default - prevents DoS)
pub const MAX_PROMPT_SIZE: usize = 100 * 1024;
/// Maximum allowed system prompt size in bytes (10KB)
pub const MAX_SYSTEM_SIZE: usize = 10 * 1024;

/// A request to an LLM
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmRequest {
    /// Tenant ID (for cache isolation)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tenant_id: Option<String>,
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
            tenant_id: None,
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
            tenant_id: None,
        }
    }

    /// Validate request sizes to prevent DoS attacks
    pub fn validate(&self) -> Result<(), LlmError> {
        if self.prompt.len() > MAX_PROMPT_SIZE {
            return Err(LlmError::InputTooLarge(self.prompt.len(), MAX_PROMPT_SIZE));
        }
        if self.system.len() > MAX_SYSTEM_SIZE {
            return Err(LlmError::InputTooLarge(self.system.len(), MAX_SYSTEM_SIZE));
        }
        Ok(())
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
    /// Merkle root of logit hashes (for cryptographic verification)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub trace_root: Option<String>,
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

/// Trait for embedding providers (text-to-vector)
#[async_trait]
pub trait EmbeddingProvider: Send + Sync + std::fmt::Debug {
    /// Generate an embedding vector for the given text
    async fn embed(&self, text: &str) -> Result<Vec<f32>, LlmError>;
}
