//! Mock LLM provider for testing

use async_trait::async_trait;
use std::time::Instant;

use crate::provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// A mock LLM provider that returns predefined responses
/// Perfect for testing without needing actual LLM access
#[derive(Debug)]
pub struct MockProvider {
    /// Name of this mock
    pub name: String,
    /// Canned responses (cycles through them)
    responses: Vec<String>,
    /// Current response index
    index: std::sync::atomic::AtomicUsize,
    /// Simulated latency in ms
    latency_ms: u64,
}

impl MockProvider {
    /// Create a new mock provider with given responses
    pub fn new(responses: Vec<String>) -> Self {
        Self {
            name: "mock".to_string(),
            responses,
            index: std::sync::atomic::AtomicUsize::new(0),
            latency_ms: 50,
        }
    }

    /// Create a mock that always returns the same response
    pub fn constant(response: &str) -> Self {
        Self::new(vec![response.to_string()])
    }

    /// Create a mock for adversarial testing (alternates agree/disagree)
    pub fn adversarial() -> Self {
        Self::new(vec![
            "I agree with this assessment. The reasoning is sound.".to_string(),
            "I disagree. There are several issues: 1) The logic is flawed, 2) Missing evidence."
                .to_string(),
        ])
    }

    /// Create a smart mock that responds based on prompt content
    pub fn smart() -> Self {
        Self {
            name: "smart-mock".to_string(),
            responses: vec![],
            index: std::sync::atomic::AtomicUsize::new(0),
            latency_ms: 50,
        }
    }

    fn generate_smart_response(&self, request: &LlmRequest) -> String {
        let prompt_lower = request.prompt.to_lowercase();

        // Detect if this is a challenge/verification request
        if prompt_lower.contains("challenge")
            || prompt_lower.contains("verify")
            || prompt_lower.contains("critique")
        {
            return "After careful analysis, I found the following concerns:\n\
                 1. The claim requires additional evidence\n\
                 2. There may be alternative interpretations\n\
                 3. Confidence level: 70%\n\n\
                 Recommendation: Proceed with caution."
                .to_string();
        }

        // Detect if this is a research/exploration request
        if prompt_lower.contains("research")
            || prompt_lower.contains("explore")
            || prompt_lower.contains("analyze")
        {
            return "Based on my analysis:\n\n\
                 ## Key Findings\n\
                 1. Primary insight discovered\n\
                 2. Supporting evidence found\n\
                 3. Potential implications identified\n\n\
                 ## Confidence: 85%\n\n\
                 This analysis is based on available information."
                .to_string();
        }

        // Detect if this is a summary request
        if prompt_lower.contains("summarize") || prompt_lower.contains("summary") {
            return "Summary: The key points are consolidated into a concise overview.".to_string();
        }

        // Default intelligent response
        format!(
            "I understand you're asking about: \"{}\"\n\n\
             Here's my response based on the context provided:\n\
             - The request has been processed\n\
             - Analysis complete\n\
             - Ready for further instructions",
            &request.prompt[..request.prompt.len().min(50)]
        )
    }
}

#[async_trait]
impl LlmProvider for MockProvider {
    fn name(&self) -> &str {
        &self.name
    }

    async fn is_available(&self) -> bool {
        true // Mock is always available
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let start = Instant::now();

        // Simulate latency
        tokio::time::sleep(std::time::Duration::from_millis(self.latency_ms)).await;

        let content = if self.responses.is_empty() {
            self.generate_smart_response(&request)
        } else {
            // Cycle through canned responses
            let idx = self
                .index
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            self.responses[idx % self.responses.len()].clone()
        };

        Ok(LlmResponse {
            content,
            model: self.name.clone(),
            tokens_used: Some((request.prompt.len() / 4) as u32 + 100),
            latency_ms: start.elapsed().as_millis() as u64,
            trace_root: None,
        })
    }
}

#[async_trait]
impl crate::provider::EmbeddingProvider for MockProvider {
    async fn embed(&self, _text: &str) -> Result<Vec<f32>, LlmError> {
        // Return a zeroed vector of dimension 1536 (common for OpenAI)
        Ok(vec![0.0; 1536])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_provider() {
        let mock = MockProvider::constant("Hello, world!");
        let response = mock.ask("test").await.unwrap();
        assert_eq!(response, "Hello, world!");
    }

    #[tokio::test]
    async fn test_smart_mock() {
        let mock = MockProvider::smart();
        let response = mock.ask("Please analyze this data").await.unwrap();
        assert!(response.contains("Key Findings"));
    }
}
