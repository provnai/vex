//! # VEX LLM
//!
//! LLM provider integrations for VEX agents.
//!
//! Supports multiple backends:
//! - DeepSeek (API)
//! - OpenAI (API)
//! - Ollama (local, free)
//! - Mock (for testing)

pub mod provider;
pub mod config;
pub mod deepseek;
pub mod openai;
pub mod ollama;
pub mod mock;
pub mod rate_limit;
pub mod metrics;
pub mod tool;

pub use provider::{LlmProvider, LlmRequest, LlmResponse, LlmError};
pub use config::{LlmConfig, VexConfig, ConfigError};
pub use deepseek::DeepSeekProvider;
pub use openai::OpenAIProvider;
pub use ollama::OllamaProvider;
pub use mock::MockProvider;
pub use rate_limit::{RateLimiter, RateLimitConfig, RateLimitError, RateLimitedProvider};
pub use metrics::{Metrics, MetricsSnapshot, Timer, Span, global_metrics};
pub use tool::ToolDefinition;

