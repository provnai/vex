//! # VEX LLM
//!
//! LLM provider integrations for VEX agents.
//!
//! ## Supported Backends
//!
//! | Provider | Type | Key Required |
//! |----------|------|--------------|
//! | DeepSeek | API | `DEEPSEEK_API_KEY` |
//! | Mistral | API | `MISTRAL_API_KEY` |
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
//! ## With Mistral
//!
//! ```rust,ignore
//! use vex_llm::MistralProvider;
//!
//! let api_key = std::env::var("MISTRAL_API_KEY").unwrap();
//! let llm = MistralProvider::small(&api_key); // or large(), medium(), codestral()
//!
//! let response = llm.ask("Explain Merkle trees").await.unwrap();
//! ```
//!
//! ## Rate Limiting
//!
//! ```rust
//! use vex_llm::{RateLimiter, RateLimitConfig};
//!
//! let limiter = RateLimiter::new(RateLimitConfig::default());
//!
//! // Check if request is allowed (in async context)
//! // limiter.try_acquire("user123").await.unwrap();
//! ```

pub mod config;
pub mod deepseek;
pub mod mcp;
pub mod metrics;
pub mod mistral;
pub mod mock;
pub mod ollama;
pub mod openai;
pub mod provider;
pub mod rate_limit;
pub mod resilient_provider;
pub mod cached_provider;
pub mod streaming_tool;
pub mod tool;
pub mod tool_error;
pub mod tool_executor;
pub mod tool_result;
pub mod tools;

pub use config::{ConfigError, LlmConfig, VexConfig};
pub use deepseek::DeepSeekProvider;
pub use metrics::{global_metrics, Metrics, MetricsSnapshot, Span, Timer};
pub use mistral::MistralProvider;
pub use mock::MockProvider;
pub use ollama::OllamaProvider;
pub use openai::OpenAIProvider;
pub use provider::{LlmError, LlmProvider, LlmRequest, LlmResponse};
pub use rate_limit::{RateLimitConfig, RateLimitError, RateLimitedProvider, RateLimiter};
pub use resilient_provider::{CircuitState, LlmCircuitConfig, ResilientProvider};
pub use cached_provider::{CachedProvider, LlmCacheConfig};
pub use streaming_tool::{StreamConfig, StreamingTool, ToolChunk, ToolStream};
pub use tool::{Capability, Tool, ToolDefinition, ToolRegistry};
pub use tool_error::ToolError;
pub use tool_executor::ToolExecutor;
pub use tool_result::ToolResult;
pub use tools::{CalculatorTool, DateTimeTool, HashTool, JsonPathTool, RegexTool, UuidTool};
