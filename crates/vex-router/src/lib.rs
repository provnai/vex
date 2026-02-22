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
pub mod cache;
pub mod classifier;
pub mod compress;
pub mod config;
pub mod guardrails;
pub mod models;
pub mod observability;
pub mod router;

// Re-export key types for easy use
pub use classifier::{QueryClassifier, QueryComplexity};
pub use config::{Config, ModelCapability, ModelConfig, RoutingStrategy};
pub use models::{Model, ModelPool};
pub use router::{
    Router, RouterBuilder, RouterConfig, RouterError, RoutingDecision,
    RoutingStrategy as RouterStrategy,
};
pub use vex_llm::{LlmError, LlmProvider, LlmRequest, LlmResponse};

// Re-export compression types
pub use compress::{CompressedPrompt, CompressionLevel, PromptCompressor};

// Re-export guardrails types
pub use guardrails::{GuardrailResult, Guardrails, Violation, ViolationCategory};

// Re-export observability types
pub use observability::{Observability, ObservabilitySummary, SavingsReport};

// Re-export cache types
pub use cache::SemanticCache;

#[cfg(feature = "standalone")]
pub mod gateway;

#[cfg(feature = "standalone")]
pub use gateway::Server;
