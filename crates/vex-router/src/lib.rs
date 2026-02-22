//! VEX Router - Intelligent LLM Routing for VEX Protocol
//!
//! A drop-in replacement for vex-llm providers that intelligently routes
//! queries to the most appropriate LLM based on complexity, cost, or quality.
//!
//! ## Quick Start
//!
//! ```rust
//! use vex_router::{Router, RoutingStrategy};
//!
//! #[tokio::main]
//! async fn main() {
//!     let router = Router::builder()
//!         .strategy(RoutingStrategy::Auto)
//!         .build();
//!
//!     let response = router.ask("What is 2+2?").await?;
//!     println!("{}", response);
//! }
//! ```
//!
//! ## For VEX Integration
//!
//! When integrated with VEX, this implements the `LlmProvider` trait
//! for drop-in replacement with vex-llm providers.

// Public modules
pub mod config;
pub mod classifier;
pub mod cache;
pub mod compress;
pub mod guardrails;
pub mod observability;
pub mod models;
pub mod router;

// Re-export key types for easy use
pub use config::{Config, RoutingStrategy, ModelConfig, ModelCapability};
pub use classifier::{QueryClassifier, QueryComplexity};
pub use models::{ModelPool, Model};
pub use router::{Router, RouterBuilder, RouterConfig, RouterError, RoutingDecision, 
                RoutingStrategy as RouterStrategy};
pub use vex_llm::{LlmRequest, LlmResponse, LlmError, LlmProvider};

// Re-export compression types
pub use compress::{CompressionLevel, PromptCompressor, CompressedPrompt};

// Re-export guardrails types
pub use guardrails::{Guardrails, GuardrailResult, Violation, ViolationCategory};

// Re-export observability types
pub use observability::{Observability, ObservabilitySummary, SavingsReport};

// Re-export cache types
pub use cache::SemanticCache;

#[cfg(feature = "standalone")]
pub mod gateway;

#[cfg(feature = "standalone")]
pub use gateway::Server;
