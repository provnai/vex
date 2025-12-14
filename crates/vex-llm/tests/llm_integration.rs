//! Integration tests that require a real LLM API
//!
//! These tests are marked with #[ignore] and require environment variables:
//! - DEEPSEEK_API_KEY for DeepSeek tests
//! - OPENAI_API_KEY for OpenAI tests
//!
//! Run with: cargo test -p vex-llm --test llm_integration -- --ignored

use vex_llm::{DeepSeekProvider, OpenAIProvider, LlmProvider, LlmRequest};

/// Test DeepSeek provider with real API
#[tokio::test]
#[ignore = "Requires DEEPSEEK_API_KEY"]
async fn test_deepseek_real_request() {
    let api_key = std::env::var("DEEPSEEK_API_KEY")
        .expect("DEEPSEEK_API_KEY must be set for this test");

    let provider = DeepSeekProvider::chat(&api_key);

    // Check availability
    assert!(provider.is_available().await, "DeepSeek should be available");

    // Make a simple request
    let request = LlmRequest {
        prompt: "What is 2 + 2? Answer with just the number.".to_string(),
        system: "You are a helpful assistant. Be extremely concise.".to_string(),
        temperature: 0.0,
        max_tokens: 10,
    };

    let response = provider.complete(request).await;
    assert!(response.is_ok(), "Request should succeed: {:?}", response);

    let response = response.unwrap();
    assert!(!response.content.is_empty(), "Response should have content");
    assert!(response.content.contains("4"), "Response should contain '4'");
    assert!(response.latency_ms > 0, "Should have latency recorded");

    println!("DeepSeek response: {}", response.content);
    println!("Latency: {}ms", response.latency_ms);
    println!("Tokens: {:?}", response.tokens_used);
}

/// Test OpenAI provider with real API
#[tokio::test]
#[ignore = "Requires OPENAI_API_KEY"]
async fn test_openai_real_request() {
    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("OPENAI_API_KEY must be set for this test");

    let provider = OpenAIProvider::gpt35(&api_key);

    // Check availability
    assert!(provider.is_available().await, "OpenAI should be available");

    // Make a simple request
    let response = provider.ask("Say 'hello' in one word").await;
    assert!(response.is_ok(), "Request should succeed: {:?}", response);

    let content = response.unwrap();
    assert!(!content.is_empty(), "Response should have content");

    println!("OpenAI response: {}", content);
}

/// Test error handling with invalid API key
#[tokio::test]
#[ignore = "Makes real API call"]
async fn test_invalid_api_key() {
    let provider = DeepSeekProvider::chat("invalid-key-12345");

    // Should fail with auth error
    let response = provider.ask("Hello").await;
    assert!(response.is_err(), "Should fail with invalid key");

    let err = response.unwrap_err();
    println!("Expected error: {:?}", err);
}

/// Test mock provider works correctly
#[tokio::test]
async fn test_mock_provider() {
    use vex_llm::MockProvider;

    let mock = MockProvider::smart();

    let response = mock.ask("What is 2+2?").await;
    assert!(response.is_ok());

    let content = response.unwrap();
    assert!(!content.is_empty());
    println!("Mock response: {}", content);
}

/// Test metrics are recorded
#[tokio::test]
async fn test_metrics_recording() {
    use vex_llm::{MockProvider, global_metrics};

    let mock = MockProvider::smart();

    // Make some calls
    let _ = mock.ask("Test 1").await;
    let _ = mock.ask("Test 2").await;

    // Check metrics
    let snapshot = global_metrics().snapshot();
    println!("LLM Calls: {}", snapshot.llm_calls);
}
