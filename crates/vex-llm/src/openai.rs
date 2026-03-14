//! OpenAI LLM provider — thin wrapper over `OpenAICompatibleProvider`

use async_trait::async_trait;

use crate::openai_compat::OpenAICompatibleProvider;
use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// OpenAI provider
#[derive(Debug)]
pub struct OpenAIProvider {
    inner: OpenAICompatibleProvider,
}

impl OpenAIProvider {
    /// Create a new OpenAI provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                api_key,
                model,
                "https://api.openai.com",
                "openai",
            ),
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
        self.inner.name()
    }

    async fn is_available(&self) -> bool {
        self.inner.is_available().await
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.inner.complete(request).await
    }
}
