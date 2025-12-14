//! # VEX LLM
//!
//! LLM provider integrations for VEX agents.
//!
//! ## Supported Backends
//!
//! | Provider | Type | Key Required |
//! |----------|------|--------------|
//! | DeepSeek | API | `DEEPSEEK_API_KEY` |
//! | OpenAI | API | `OPENAI_API_KEY` |
//! | Ollama | Local | None |
//! | Mock | Testing | None |
//!
//! ## Quick Start
//!
//! ```rust
//! use vex_llm::{MockProvider, LlmProvider};
//!
//! #[tokio::main]
//! async fn main() {
//!     // Use mock provider for testing
//!     let llm = MockProvider::smart();
//!     
//!     // Ask a question
//!     let response = llm.ask("What is quantum computing?").await.unwrap();
//!     println!("{}", response);
//! }
//! ```
//!
//! ## With DeepSeek
//!
//! ```rust,ignore
//! use vex_llm::DeepSeekProvider;
//!
//! let api_key = std::env::var("DEEPSEEK_API_KEY").unwrap();
//! let llm = DeepSeekProvider::new(api_key);
//!
//! let response = llm.ask("Explain Merkle trees").await.unwrap();
//! ```
//!
//! ## Rate Limiting
//!
//! ```rust
//! use vex_llm::{RateLimiter, RateLimitConfig};
//!
//! let limiter = RateLimiter::new(RateLimitConfig {
//!     requests_per_second: 10,
//!     burst_size: 20,
//! });
//!
//! // Check if request is allowed
//! limiter.try_acquire("user123").await.unwrap();
//! ```

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

