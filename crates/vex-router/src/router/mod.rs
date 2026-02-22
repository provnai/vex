//! Router - Core routing logic for VEX

use serde::{Deserialize, Serialize};
#[allow(unused_imports)]
use std::sync::Arc;
use thiserror::Error;

use crate::classifier::{QueryClassifier, QueryComplexity};
use crate::compress::CompressionLevel;
use crate::models::{Model, ModelPool};
use crate::observability::Observability;

/// Routing strategy (re-exported from config)
pub use crate::config::RoutingStrategy;

/// A routing decision
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingDecision {
    pub model_id: String,
    pub estimated_cost: f64,
    pub estimated_latency_ms: u64,
    pub estimated_savings: f64,
    pub reason: String,
}

/// Router configuration
#[derive(Debug, Clone)]
pub struct RouterConfig {
    pub strategy: RoutingStrategy,
    pub quality_threshold: f64,
    pub max_cost_per_request: f64,
    pub max_latency_ms: u64,
    pub cache_enabled: bool,
    pub guardrails_enabled: bool,
    pub compression_level: CompressionLevel,
}

impl Default for RouterConfig {
    fn default() -> Self {
        Self {
            strategy: RoutingStrategy::Auto,
            quality_threshold: 0.85,
            max_cost_per_request: 1.0,
            max_latency_ms: 10000,
            cache_enabled: true,
            guardrails_enabled: true,
            compression_level: CompressionLevel::Balanced,
        }
    }
}

/// Router errors
#[derive(Debug, Error)]
pub enum RouterError {
    #[error("No models available")]
    NoModelsAvailable,
    #[error("Request failed: {0}")]
    RequestFailed(String),
    #[error("All models failed")]
    AllModelsFailed,
    #[error("Guardrails blocked request")]
    GuardrailsBlocked,
}

/// The main Router - implements LlmProvider trait for VEX
#[derive(Debug)]
pub struct Router {
    pool: ModelPool,
    classifier: QueryClassifier,
    config: RouterConfig,
    observability: Observability,
}

impl Router {
    /// Create a new router with default settings
    pub fn new() -> Self {
        Self {
            pool: ModelPool::default(),
            classifier: QueryClassifier::new(),
            config: RouterConfig::default(),
            observability: Observability::default(),
        }
    }

    /// Create a router with a custom configuration
    pub fn with_config(config: RouterConfig) -> Self {
        Self {
            pool: ModelPool::default(),
            classifier: QueryClassifier::new(),
            config,
            observability: Observability::default(),
        }
    }

    /// Get a builder for configuration
    pub fn builder() -> RouterBuilder {
        RouterBuilder::new()
    }

    /// Route a query and return a decision (without executing)
    pub fn route(&self, prompt: &str, system: &str) -> Result<RoutingDecision, RouterError> {
        let mut complexity = self.classifier.classify(prompt);

        // ADVERSARIAL ROUTING: If system prompt implies an attacker/shadow role,
        // bump the complexity/quality requirements to ensure a strong adversary.
        let system_lower = system.to_lowercase();
        if system_lower.contains("shadow")
            || system_lower.contains("adversarial")
            || system_lower.contains("red agent")
        {
            complexity.score = (complexity.score + 0.4).min(1.0);
            complexity.capabilities.push("adversarial".to_string());
        }

        self.route_with_complexity(&complexity)
    }

    /// Route with pre-computed complexity
    pub fn route_with_complexity(
        &self,
        complexity: &QueryComplexity,
    ) -> Result<RoutingDecision, RouterError> {
        if self.pool.is_empty() {
            return Err(RouterError::NoModelsAvailable);
        }

        match self.config.strategy {
            RoutingStrategy::Auto | RoutingStrategy::Balanced => self.route_auto(complexity),
            RoutingStrategy::CostOptimized => self.route_cost_optimized(complexity),
            RoutingStrategy::QualityOptimized => self.route_quality_optimized(complexity),
            RoutingStrategy::LatencyOptimized => self.route_latency_optimized(complexity),
            RoutingStrategy::Custom => {
                // Fall back to auto for custom
                self.route_auto(complexity)
            }
        }
    }

    /// Execute a query through the router
    pub async fn execute(&self, prompt: &str, system: &str) -> Result<String, RouterError> {
        let decision = self.route(prompt, system)?;

        // For now, return a mock response
        // In VEX integration, this would call the actual LLM
        Ok(format!(
            "[vex-router: {}] Query routed based on complexity: {:.2}, Role: {}, Estimated savings: {:.0}%",
            decision.model_id,
            0.5,
            if system.to_lowercase().contains("shadow") { "Adversarial" } else { "Primary" },
            decision.estimated_savings
        ))
    }

    /// Convenience method - ask a question
    pub async fn ask(&self, prompt: &str) -> Result<String, RouterError> {
        self.execute(prompt, "").await
    }

    // =========================================================================
    // Routing Strategies
    // =========================================================================

    fn route_auto(&self, complexity: &QueryComplexity) -> Result<RoutingDecision, RouterError> {
        // Simple heuristic: low complexity = cheap model, high complexity = premium
        let model = if complexity.score < 0.3 {
            self.pool.get_cheapest()
        } else if complexity.score < 0.7 {
            self.pool.get_medium()
        } else {
            self.pool.get_best()
        };

        let model = model.ok_or(RouterError::NoModelsAvailable)?;

        let savings = if complexity.score < 0.3 {
            95.0
        } else if complexity.score < 0.7 {
            60.0
        } else {
            20.0
        };

        Ok(RoutingDecision {
            model_id: model.id.clone(),
            estimated_cost: model.config.input_cost,
            estimated_latency_ms: model.config.latency_ms,
            estimated_savings: savings,
            reason: format!(
                "Auto-selected based on complexity score: {:.2}",
                complexity.score
            ),
        })
    }

    fn route_cost_optimized(
        &self,
        _complexity: &QueryComplexity,
    ) -> Result<RoutingDecision, RouterError> {
        // Find cheapest model that meets quality threshold
        let mut models: Vec<&Model> = self.pool.models.iter().collect();
        models.sort_by(|a, b| {
            a.config
                .input_cost
                .partial_cmp(&b.config.input_cost)
                .unwrap()
        });

        for model in models {
            let meets_quality = model.config.quality_score >= self.config.quality_threshold;
            if meets_quality {
                return Ok(RoutingDecision {
                    model_id: model.id.clone(),
                    estimated_cost: model.config.input_cost,
                    estimated_latency_ms: model.config.latency_ms,
                    estimated_savings: 80.0,
                    reason: "Cost-optimized: cheapest model meeting quality threshold".to_string(),
                });
            }
        }

        Err(RouterError::NoModelsAvailable)
    }

    fn route_quality_optimized(
        &self,
        _complexity: &QueryComplexity,
    ) -> Result<RoutingDecision, RouterError> {
        let model = self.pool.get_best().ok_or(RouterError::NoModelsAvailable)?;

        Ok(RoutingDecision {
            model_id: model.id.clone(),
            estimated_cost: model.config.input_cost,
            estimated_latency_ms: model.config.latency_ms,
            estimated_savings: 0.0,
            reason: "Quality-optimized: selected best available model".to_string(),
        })
    }

    fn route_latency_optimized(
        &self,
        _complexity: &QueryComplexity,
    ) -> Result<RoutingDecision, RouterError> {
        let mut models: Vec<&Model> = self.pool.models.iter().collect();
        models.sort_by(|a, b| a.config.latency_ms.cmp(&b.config.latency_ms));

        let model = models.first().ok_or(RouterError::NoModelsAvailable)?;

        Ok(RoutingDecision {
            model_id: model.id.clone(),
            estimated_cost: model.config.input_cost,
            estimated_latency_ms: model.config.latency_ms,
            estimated_savings: 50.0,
            reason: "Latency-optimized: fastest model".to_string(),
        })
    }

    /// Get the current configuration
    pub fn config(&self) -> &RouterConfig {
        &self.config
    }

    /// Get the model pool
    pub fn pool(&self) -> &ModelPool {
        &self.pool
    }

    /// Get the observability metrics
    pub fn observability(&self) -> &Observability {
        &self.observability
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for Router
#[derive(Debug)]
pub struct RouterBuilder {
    config: RouterConfig,
    custom_models: Vec<crate::config::ModelConfig>,
}

impl RouterBuilder {
    pub fn new() -> Self {
        Self {
            config: RouterConfig::default(),
            custom_models: Vec::new(),
        }
    }

    pub fn strategy(mut self, strategy: RoutingStrategy) -> Self {
        self.config.strategy = strategy;
        self
    }

    pub fn quality_threshold(mut self, threshold: f64) -> Self {
        self.config.quality_threshold = threshold;
        self
    }

    pub fn max_cost(mut self, cost: f64) -> Self {
        self.config.max_cost_per_request = cost;
        self
    }

    pub fn cache_enabled(mut self, enabled: bool) -> Self {
        self.config.cache_enabled = enabled;
        self
    }

    pub fn guardrails_enabled(mut self, enabled: bool) -> Self {
        self.config.guardrails_enabled = enabled;
        self
    }

    pub fn compression_level(mut self, level: crate::compress::CompressionLevel) -> Self {
        self.config.compression_level = level;
        self
    }

    pub fn add_model(mut self, model: crate::config::ModelConfig) -> Self {
        self.custom_models.push(model);
        self
    }

    pub fn build(self) -> Router {
        let pool = if self.custom_models.is_empty() {
            ModelPool::default()
        } else {
            ModelPool::new(self.custom_models)
        };

        Router {
            pool,
            classifier: QueryClassifier::new(),
            config: self.config,
            observability: Observability::new(1000),
        }
    }
}

impl Default for RouterBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// =============================================================================
// VEX LlmProvider Trait Implementation (for VEX integration)
// =============================================================================

// Re-using official VEX LLM types
use async_trait::async_trait;
use vex_llm::{LlmError, LlmProvider, LlmRequest, LlmResponse};

#[async_trait]
impl LlmProvider for Router {
    /// Complete a request (implements vex_llm::LlmProvider::complete)
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let start = std::time::Instant::now();

        let response = self
            .execute(&request.prompt, &request.system)
            .await
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        let response_len = response.len();
        let latency = start.elapsed().as_millis() as u64;

        let decision = self
            .route(&request.prompt, &request.system)
            .map_err(|e| LlmError::RequestFailed(e.to_string()))?;

        Ok(LlmResponse {
            content: response,
            model: decision.model_id,
            tokens_used: Some(((request.prompt.len() + response_len) as f64 / 4.0) as u32),
            latency_ms: latency,
            trace_root: None,
        })
    }

    /// Check if router is available
    async fn is_available(&self) -> bool {
        !self.pool.is_empty()
    }

    /// Get provider name
    fn name(&self) -> &str {
        "vex-router"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_router_auto() {
        let router = Router::builder().strategy(RoutingStrategy::Auto).build();

        let decision = router.route("What is 2+2?", "").unwrap();
        assert!(!decision.model_id.is_empty());
    }

    #[tokio::test]
    async fn test_router_execute() {
        let router = Router::new();
        let response = router.ask("Hello").await.unwrap();
        assert!(response.contains("vex-router"));
    }

    #[test]
    fn test_router_builder() {
        let router = Router::builder()
            .strategy(RoutingStrategy::CostOptimized)
            .quality_threshold(0.9)
            .cache_enabled(false)
            .build();

        assert_eq!(router.config().strategy, RoutingStrategy::CostOptimized);
        assert_eq!(router.config().quality_threshold, 0.9);
        assert!(!router.config().cache_enabled);
    }

    #[tokio::test]
    async fn test_llm_request() {
        let request = LlmRequest::simple("test");
        assert_eq!(request.system, "You are a helpful assistant.");
        assert_eq!(request.prompt, "test");
    }
}
