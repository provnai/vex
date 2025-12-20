//! Resilient LLM provider wrapper with circuit breaker pattern
//!
//! Provides fault tolerance for LLM providers by implementing the circuit breaker
//! pattern, preventing cascading failures when external providers are unavailable.
//!
//! # 2025 Best Practices
//! - Three states: Closed (normal), Open (failing fast), Half-Open (testing recovery)
//! - Configurable thresholds and timeouts
//! - Automatic recovery testing after cooldown period

use async_trait::async_trait;
use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

use crate::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Circuit breaker state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CircuitState {
    /// Normal operation - requests pass through
    Closed,
    /// Circuit tripped - requests fail immediately  
    Open,
    /// Testing recovery - limited requests allowed
    HalfOpen,
}

/// Configuration for the LLM circuit breaker
#[derive(Debug, Clone)]
pub struct LlmCircuitConfig {
    /// Number of failures before opening circuit
    pub failure_threshold: u32,
    /// Number of successes in half-open to close circuit
    pub success_threshold: u32,
    /// Time to wait before testing recovery
    pub reset_timeout: Duration,
}

impl Default for LlmCircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 2,
            reset_timeout: Duration::from_secs(30),
        }
    }
}

impl LlmCircuitConfig {
    /// Conservative settings for production LLM providers
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 3,
            reset_timeout: Duration::from_secs(60),
        }
    }
}

/// Internal circuit breaker state
#[derive(Debug)]
struct CircuitBreakerState {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure: Option<Instant>,
}

/// Resilient LLM provider that wraps any provider with circuit breaker resilience
#[derive(Debug)]
pub struct ResilientProvider<P: LlmProvider> {
    inner: Arc<P>,
    config: LlmCircuitConfig,
    cb_state: RwLock<CircuitBreakerState>,
    total_requests: AtomicU64,
    total_failures: AtomicU64,
    circuit_opens: AtomicU32,
}

impl<P: LlmProvider> ResilientProvider<P> {
    /// Create a resilient wrapper around an LLM provider
    pub fn new(provider: P, config: LlmCircuitConfig) -> Self {
        Self {
            inner: Arc::new(provider),
            config,
            cb_state: RwLock::new(CircuitBreakerState {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure: None,
            }),
            total_requests: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            circuit_opens: AtomicU32::new(0),
        }
    }

    /// Create with default (conservative) config
    pub fn wrap(provider: P) -> Self {
        Self::new(provider, LlmCircuitConfig::conservative())
    }

    /// Get current circuit state
    pub async fn circuit_state(&self) -> CircuitState {
        self.cb_state.read().await.state
    }

    /// Get circuit statistics
    pub fn stats(&self) -> (u64, u64, u32) {
        (
            self.total_requests.load(Ordering::Relaxed),
            self.total_failures.load(Ordering::Relaxed),
            self.circuit_opens.load(Ordering::Relaxed),
        )
    }

    async fn record_success(&self) {
        let mut state = self.cb_state.write().await;
        state.failure_count = 0;
        
        if state.state == CircuitState::HalfOpen {
            state.success_count += 1;
            if state.success_count >= self.config.success_threshold {
                state.state = CircuitState::Closed;
                state.success_count = 0;
                tracing::info!(provider = %self.inner.name(), "Circuit closed - provider recovered");
            }
        }
    }

    async fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        let mut state = self.cb_state.write().await;
        state.failure_count += 1;
        state.last_failure = Some(Instant::now());
        
        if state.state == CircuitState::HalfOpen {
            // Any failure in half-open goes back to open
            state.state = CircuitState::Open;
            self.circuit_opens.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(provider = %self.inner.name(), "Circuit re-opened - recovery test failed");
        } else if state.failure_count >= self.config.failure_threshold {
            state.state = CircuitState::Open;
            self.circuit_opens.fetch_add(1, Ordering::Relaxed);
            tracing::warn!(
                provider = %self.inner.name(),
                failures = state.failure_count,
                "Circuit opened - failure threshold exceeded"
            );
        }
    }

    async fn check_circuit(&self) -> Result<(), LlmError> {
        let mut state = self.cb_state.write().await;
        
        match state.state {
            CircuitState::Closed => Ok(()),
            CircuitState::Open => {
                // Check if reset timeout has passed
                if let Some(last_failure) = state.last_failure {
                    if last_failure.elapsed() >= self.config.reset_timeout {
                        state.state = CircuitState::HalfOpen;
                        state.success_count = 0;
                        tracing::info!(provider = %self.inner.name(), "Circuit half-open - testing recovery");
                        return Ok(());
                    }
                }
                Err(LlmError::NotAvailable)
            }
            CircuitState::HalfOpen => Ok(()),
        }
    }
}

#[async_trait]
impl<P: LlmProvider + 'static> LlmProvider for ResilientProvider<P> {
    fn name(&self) -> &str {
        // Return a static descriptor since we can't easily compose names
        "resilient"
    }

    async fn is_available(&self) -> bool {
        self.check_circuit().await.is_ok() && self.inner.is_available().await
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        // Check circuit state
        self.check_circuit().await?;
        
        // Execute request
        match self.inner.complete(request).await {
            Ok(response) => {
                self.record_success().await;
                Ok(response)
            }
            Err(e) => {
                // Only count as failure for connection/availability issues, not validation
                match &e {
                    LlmError::ConnectionFailed(_) | 
                    LlmError::NotAvailable | 
                    LlmError::RateLimited => {
                        self.record_failure().await;
                    }
                    _ => {}
                }
                Err(e)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[tokio::test]
    async fn test_resilient_provider_passes_through() {
        let mock = MockProvider::smart();
        let resilient = ResilientProvider::wrap(mock);
        
        let result = resilient.ask("test").await;
        assert!(result.is_ok());
        assert_eq!(resilient.circuit_state().await, CircuitState::Closed);
    }
}
