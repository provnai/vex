//! Mistral AI LLM provider — thin wrapper over `OpenAICompatibleProvider`

use async_trait::async_trait;

use crate::openai_compat::OpenAICompatibleProvider;
use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Mistral AI provider for inference
#[derive(Debug)]
pub struct MistralProvider {
    inner: OpenAICompatibleProvider,
}

impl MistralProvider {
    /// Create a new Mistral provider
    pub fn new(api_key: &str, model: &str) -> Self {
        Self {
            inner: OpenAICompatibleProvider::new(
                api_key,
                model,
                "https://api.mistral.ai",
                "mistral",
            ),
        }
    }

    /// Create with Mistral Large 3
    pub fn large(api_key: &str) -> Self {
        Self::new(api_key, "mistral-large-latest")
    }

    /// Create with Mistral Medium 3.1
    pub fn medium(api_key: &str) -> Self {
        Self::new(api_key, "mistral-medium-latest")
    }

    /// Create with Mistral Small 3.2
    pub fn small(api_key: &str) -> Self {
        Self::new(api_key, "mistral-small-latest")
    }

    /// Create with Codestral
    pub fn codestral(api_key: &str) -> Self {
        Self::new(api_key, "codestral-latest")
    }

    /// Create with Devstral
    pub fn devstral(api_key: &str) -> Self {
        Self::new(api_key, "devstral-small-latest")
    }

    /// Create with Ministral 8B
    pub fn ministral_8b(api_key: &str) -> Self {
        Self::new(api_key, "ministral-8b-latest")
    }

    /// Create with Ministral 3B
    pub fn ministral_3b(api_key: &str) -> Self {
        Self::new(api_key, "ministral-3b-latest")
    }

    /// Create with Pixtral Large
    pub fn pixtral(api_key: &str) -> Self {
        Self::new(api_key, "pixtral-large-latest")
    }

    /// Create with Mistral Nemo 12B
    pub fn nemo(api_key: &str) -> Self {
        Self::new(api_key, "open-mistral-nemo")
    }

    /// Set a custom base URL
    pub fn with_base_url(mut self, base_url: &str) -> Self {
        self.inner = self.inner.with_base_url(base_url);
        self
    }
}

#[async_trait]
impl LlmProvider for MistralProvider {
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
