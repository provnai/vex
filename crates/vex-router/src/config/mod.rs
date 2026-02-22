//! Configuration module for SmartRouter

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Routing strategy to use
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum RoutingStrategy {
    /// Automatically pick best model (default)
    #[default]
    Auto,
    /// Minimize cost, maintain quality threshold
    CostOptimized,
    /// Maximize quality, ignore cost
    QualityOptimized,
    /// Minimize latency
    LatencyOptimized,
    /// Balanced approach
    Balanced,
    /// User-defined rules
    Custom,
}

/// Configuration for a specific model
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelConfig {
    /// Model identifier (e.g., "gpt-4o", "claude-3-haiku")
    pub id: String,
    /// Display name
    pub name: String,
    /// Cost per 1M input tokens
    pub input_cost: f64,
    /// Cost per 1M output tokens
    pub output_cost: f64,
    /// Average latency in ms
    pub latency_ms: u64,
    /// Quality score (0-1)
    pub quality_score: f64,
    /// Capabilities this model excels at
    pub capabilities: Vec<ModelCapability>,
    /// Is this a premium model (used as fallback)?
    pub is_premium: bool,
}

/// Model capability tags
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ModelCapability {
    Code,
    Reasoning,
    Creative,
    Math,
    Analysis,
    Summarization,
    Translation,
    Chat,
    General,
}

/// Main SmartRouter configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Server configuration
    pub server: ServerConfig,
    /// Available models
    pub models: Vec<ModelConfig>,
    /// Default routing strategy
    pub default_strategy: RoutingStrategy,
    /// API keys for model providers (encrypted in production)
    pub api_keys: HashMap<String, String>,
    /// Quality threshold (0-1) for cost-optimized routing
    pub quality_threshold: f64,
    /// Maximum cost per 1K tokens allowed
    pub max_cost_per_1k: f64,
    /// Maximum latency allowed (ms)
    pub max_latency_ms: u64,
    /// Enable learning system
    pub learning_enabled: bool,
    /// Cache responses
    pub cache_enabled: bool,
    /// Rate limit configuration
    pub rate_limit: RateLimitConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub workers: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateLimitConfig {
    pub requests_per_minute: u32,
    pub requests_per_day: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            server: ServerConfig {
                host: "0.0.0.0".to_string(),
                port: 3000,
                workers: 4,
            },
            models: default_models(),
            default_strategy: RoutingStrategy::Auto,
            api_keys: HashMap::new(),
            quality_threshold: 0.85,
            max_cost_per_1k: 1.0,
            max_latency_ms: 5000,
            learning_enabled: true,
            cache_enabled: true,
            rate_limit: RateLimitConfig {
                requests_per_minute: 1000,
                requests_per_day: 100000,
            },
        }
    }
}

pub fn default_models() -> Vec<ModelConfig> {
    vec![
        ModelConfig {
            id: "gpt-4o".to_string(),
            name: "GPT-4o".to_string(),
            input_cost: 15.0,
            output_cost: 15.0,
            latency_ms: 3000,
            quality_score: 0.98,
            capabilities: vec![
                ModelCapability::Reasoning,
                ModelCapability::Code,
                ModelCapability::Creative,
                ModelCapability::Analysis,
            ],
            is_premium: true,
        },
        ModelConfig {
            id: "gpt-4o-mini".to_string(),
            name: "GPT-4o Mini".to_string(),
            input_cost: 0.60,
            output_cost: 0.60,
            latency_ms: 1000,
            quality_score: 0.85,
            capabilities: vec![
                ModelCapability::Chat,
                ModelCapability::Summarization,
                ModelCapability::General,
            ],
            is_premium: false,
        },
        ModelConfig {
            id: "claude-3-5-sonnet".to_string(),
            name: "Claude 3.5 Sonnet".to_string(),
            input_cost: 3.0,
            output_cost: 15.0,
            latency_ms: 2500,
            quality_score: 0.97,
            capabilities: vec![
                ModelCapability::Reasoning,
                ModelCapability::Creative,
                ModelCapability::Analysis,
            ],
            is_premium: true,
        },
        ModelConfig {
            id: "claude-3-haiku".to_string(),
            name: "Claude 3 Haiku".to_string(),
            input_cost: 0.25,
            output_cost: 1.25,
            latency_ms: 800,
            quality_score: 0.82,
            capabilities: vec![
                ModelCapability::Chat,
                ModelCapability::Summarization,
                ModelCapability::General,
            ],
            is_premium: false,
        },
        ModelConfig {
            id: "llama-3-70b".to_string(),
            name: "Llama 3 70B".to_string(),
            input_cost: 0.90,
            output_cost: 0.90,
            latency_ms: 4000,
            quality_score: 0.88,
            capabilities: vec![
                ModelCapability::Code,
                ModelCapability::General,
            ],
            is_premium: false,
        },
    ]
}
