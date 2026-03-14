//! DeepSeek LLM provider — thin wrapper over `OpenAICompatibleProvider`

use async_trait::async_trait;

use crate::openai_compat::OpenAICompatibleProvider;
use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// DeepSeek provider for inference
#[derive(Debug, Clone)]
pub struct DeepSeekProvider {
    inner: OpenAICompatibleProvider,
}

impl DeepSeekProvider {
    /// Create a new DeepSeek provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                api_key,
                model,
                "https://api.deepseek.com",
                "deepseek",
            ),
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
        self.inner.name()
    }

    async fn is_available(&self) -> bool {
        self.inner.is_available().await
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.inner.complete(request).await
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
