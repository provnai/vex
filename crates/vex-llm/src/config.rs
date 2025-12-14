//! Configuration management for VEX
//!
//! Handles API keys, provider settings, and runtime configuration.

use serde::{Deserialize, Serialize};
use std::env;

/// Error types for configuration
#[derive(Debug, thiserror::Error)]
pub enum ConfigError {
    #[error("Missing environment variable: {0}")]
    MissingEnvVar(String),
    #[error("Invalid configuration: {0}")]
    Invalid(String),
}

/// LLM provider configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmConfig {
    /// DeepSeek API key (env: DEEPSEEK_API_KEY)
    pub deepseek_api_key: Option<String>,
    /// Mistral API key (env: MISTRAL_API_KEY)
    pub mistral_api_key: Option<String>,
    /// OpenAI API key (env: OPENAI_API_KEY)
    pub openai_api_key: Option<String>,
    /// Anthropic API key (env: ANTHROPIC_API_KEY)
    pub anthropic_api_key: Option<String>,
    /// Ollama base URL (default: http://localhost:11434)
    pub ollama_url: String,
    /// Default provider
    pub default_provider: String,
    /// Default model
    pub default_model: String,
}

impl Default for LlmConfig {
    fn default() -> Self {
        Self {
            deepseek_api_key: None,
            mistral_api_key: None,
            openai_api_key: None,
            anthropic_api_key: None,
            ollama_url: "http://localhost:11434".to_string(),
            default_provider: "deepseek".to_string(),
            default_model: "deepseek-chat".to_string(),
        }
    }
}

impl LlmConfig {
    /// Load configuration from environment variables
    pub fn from_env() -> Self {
        Self {
            deepseek_api_key: env::var("DEEPSEEK_API_KEY").ok(),
            mistral_api_key: env::var("MISTRAL_API_KEY").ok(),
            openai_api_key: env::var("OPENAI_API_KEY").ok(),
            anthropic_api_key: env::var("ANTHROPIC_API_KEY").ok(),
            ollama_url: env::var("OLLAMA_URL")
                .unwrap_or_else(|_| "http://localhost:11434".to_string()),
            default_provider: env::var("VEX_DEFAULT_PROVIDER")
                .unwrap_or_else(|_| "deepseek".to_string()),
            default_model: env::var("VEX_DEFAULT_MODEL")
                .unwrap_or_else(|_| "deepseek-chat".to_string()),
        }
    }

    /// Get API key for a provider
    pub fn api_key(&self, provider: &str) -> Option<&str> {
        match provider.to_lowercase().as_str() {
            "deepseek" => self.deepseek_api_key.as_deref(),
            "mistral" => self.mistral_api_key.as_deref(),
            "openai" => self.openai_api_key.as_deref(),
            "anthropic" => self.anthropic_api_key.as_deref(),
            _ => None,
        }
    }

    /// Check if a provider is configured
    pub fn is_configured(&self, provider: &str) -> bool {
        match provider.to_lowercase().as_str() {
            "deepseek" => self.deepseek_api_key.is_some(),
            "mistral" => self.mistral_api_key.is_some(),
            "openai" => self.openai_api_key.is_some(),
            "anthropic" => self.anthropic_api_key.is_some(),
            "ollama" | "mock" => true, // Always available
            _ => false,
        }
    }

    /// List available providers
    pub fn available_providers(&self) -> Vec<&str> {
        let mut providers = vec!["mock", "ollama"];
        if self.deepseek_api_key.is_some() {
            providers.push("deepseek");
        }
        if self.mistral_api_key.is_some() {
            providers.push("mistral");
        }
        if self.openai_api_key.is_some() {
            providers.push("openai");
        }
        if self.anthropic_api_key.is_some() {
            providers.push("anthropic");
        }
        providers
    }
}

/// Full VEX configuration
#[derive(Debug, Clone, Default)]
pub struct VexConfig {
    /// LLM provider settings
    pub llm: LlmConfig,
    /// Enable debug logging
    pub debug: bool,
    /// Maximum agent depth
    pub max_agent_depth: u8,
    /// Enable adversarial verification
    pub adversarial_enabled: bool,
}

impl VexConfig {
    /// Load from environment
    pub fn from_env() -> Self {
        Self {
            llm: LlmConfig::from_env(),
            debug: env::var("VEX_DEBUG")
                .map(|v| v == "1" || v == "true")
                .unwrap_or(false),
            max_agent_depth: env::var("VEX_MAX_DEPTH")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(3),
            adversarial_enabled: env::var("VEX_ADVERSARIAL")
                .map(|v| v != "0" && v != "false")
                .unwrap_or(true),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = LlmConfig::default();
        assert_eq!(config.default_provider, "deepseek");
        assert!(config.is_configured("mock"));
        assert!(config.is_configured("ollama"));
    }

    #[test]
    fn test_available_providers() {
        let config = LlmConfig::default();
        let providers = config.available_providers();
        assert!(providers.contains(&"mock"));
        assert!(providers.contains(&"ollama"));
    }
}
