//! Circuit breaker for resilient service calls

use std::sync::atomic::{AtomicU32, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

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

/// Circuit breaker configuration
#[derive(Debug, Clone)]
pub struct CircuitConfig {
    /// Failure threshold to trip circuit
    pub failure_threshold: u32,
    /// Success threshold in half-open to close
    pub success_threshold: u32,
    /// Failure threshold in half-open before re-opening (allows a few test failures)
    pub half_open_failure_threshold: u32,
    /// Time to wait in open state before testing
    pub reset_timeout: Duration,
    /// Rolling window for failures
    pub window_duration: Duration,
}

impl Default for CircuitConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            half_open_failure_threshold: 2, // Allow 1 test failure before re-opening
            reset_timeout: Duration::from_secs(30),
            window_duration: Duration::from_secs(60),
        }
    }
}

impl CircuitConfig {
    /// Conservative settings for critical services
    pub fn conservative() -> Self {
        Self {
            failure_threshold: 3,
            success_threshold: 5,
            half_open_failure_threshold: 1, // Strict: re-open on first failure
            reset_timeout: Duration::from_secs(60),
            window_duration: Duration::from_secs(120),
        }
    }

    /// Aggressive settings for non-critical services
    pub fn aggressive() -> Self {
        Self {
            failure_threshold: 10,
            success_threshold: 2,
            half_open_failure_threshold: 3, // Allow several test failures
            reset_timeout: Duration::from_secs(10),
            window_duration: Duration::from_secs(30),
        }
    }
}

/// Thread-safe circuit breaker
#[derive(Debug)]
pub struct CircuitBreaker {
    name: String,
    config: CircuitConfig,
    state: RwLock<CircuitState>,
    failure_count: AtomicU32,
    success_count: AtomicU32,
    half_open_failure_count: AtomicU32,
    last_failure_time: RwLock<Option<Instant>>,
    last_state_change: RwLock<Instant>,
    total_requests: AtomicU64,
    total_failures: AtomicU64,
    total_rejections: AtomicU64,
}

impl CircuitBreaker {
    /// Create a new circuit breaker
    pub fn new(name: &str, config: CircuitConfig) -> Self {
        Self {
            name: name.to_string(),
            config,
            state: RwLock::new(CircuitState::Closed),
            failure_count: AtomicU32::new(0),
            success_count: AtomicU32::new(0),
            half_open_failure_count: AtomicU32::new(0),
            last_failure_time: RwLock::new(None),
            last_state_change: RwLock::new(Instant::now()),
            total_requests: AtomicU64::new(0),
            total_failures: AtomicU64::new(0),
            total_rejections: AtomicU64::new(0),
        }
    }

    /// Check if request is allowed
    pub async fn allow(&self) -> bool {
        self.total_requests.fetch_add(1, Ordering::Relaxed);
        
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if reset timeout has passed
                let last_change = *self.last_state_change.read().await;
                if last_change.elapsed() >= self.config.reset_timeout {
                    *state = CircuitState::HalfOpen;
                    *self.last_state_change.write().await = Instant::now();
                    self.success_count.store(0, Ordering::Relaxed);
                    self.half_open_failure_count.store(0, Ordering::Relaxed);
                    tracing::info!(
                        circuit = %self.name,
                        "Circuit transitioned to HalfOpen"
                    );
                    true
                } else {
                    self.total_rejections.fetch_add(1, Ordering::Relaxed);
                    false
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited requests for testing
                true
            }
        }
    }

    /// Record a successful call
    pub async fn record_success(&self) {
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::HalfOpen => {
                let count = self.success_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.success_threshold {
                    *state = CircuitState::Closed;
                    self.failure_count.store(0, Ordering::Relaxed);
                    self.success_count.store(0, Ordering::Relaxed);
                    *self.last_state_change.write().await = Instant::now();
                    tracing::info!(
                        circuit = %self.name,
                        "Circuit recovered - now Closed"
                    );
                }
            }
            CircuitState::Closed => {
                // Reset failure count on success in closed state
                self.failure_count.store(0, Ordering::Relaxed);
            }
            _ => {}
        }
    }

    /// Record a failed call
    pub async fn record_failure(&self) {
        self.total_failures.fetch_add(1, Ordering::Relaxed);
        *self.last_failure_time.write().await = Some(Instant::now());
        
        let mut state = self.state.write().await;
        
        match *state {
            CircuitState::Closed => {
                let count = self.failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                if count >= self.config.failure_threshold {
                    *state = CircuitState::Open;
                    *self.last_state_change.write().await = Instant::now();
                    tracing::warn!(
                        circuit = %self.name,
                        failures = count,
                        "Circuit tripped - now Open"
                    );
                }
            }
            CircuitState::HalfOpen => {
                // Track failures during recovery testing
                let half_open_failures = self.half_open_failure_count.fetch_add(1, Ordering::Relaxed) + 1;
                
                if half_open_failures >= self.config.half_open_failure_threshold {
                    // Too many failures during recovery, re-open the circuit
                    *state = CircuitState::Open;
                    self.success_count.store(0, Ordering::Relaxed);
                    self.half_open_failure_count.store(0, Ordering::Relaxed);
                    *self.last_state_change.write().await = Instant::now();
                    tracing::warn!(
                        circuit = %self.name,
                        half_open_failures = half_open_failures,
                        "Circuit tripped from HalfOpen - back to Open"
                    );
                } else {
                    tracing::debug!(
                        circuit = %self.name,
                        half_open_failures = half_open_failures,
                        threshold = self.config.half_open_failure_threshold,
                        "HalfOpen failure recorded, still testing"
                    );
                }
            }
            _ => {}
        }
    }

    /// Get current state
    pub async fn state(&self) -> CircuitState {
        *self.state.read().await
    }

    /// Get statistics
    pub fn stats(&self) -> CircuitStats {
        CircuitStats {
            name: self.name.clone(),
            total_requests: self.total_requests.load(Ordering::Relaxed),
            total_failures: self.total_failures.load(Ordering::Relaxed),
            total_rejections: self.total_rejections.load(Ordering::Relaxed),
            current_failures: self.failure_count.load(Ordering::Relaxed),
            current_successes: self.success_count.load(Ordering::Relaxed),
        }
    }

    /// Execute with circuit breaker protection
    pub async fn call<F, T, E>(&self, f: F) -> Result<T, CircuitError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        if !self.allow().await {
            return Err(CircuitError::Open);
        }

        match f.await {
            Ok(result) => {
                self.record_success().await;
                Ok(result)
            }
            Err(e) => {
                self.record_failure().await;
                Err(CircuitError::Failed(e))
            }
        }
    }
}

/// Circuit breaker statistics
#[derive(Debug, Clone)]
pub struct CircuitStats {
    pub name: String,
    pub total_requests: u64,
    pub total_failures: u64,
    pub total_rejections: u64,
    pub current_failures: u32,
    pub current_successes: u32,
}

/// Circuit breaker error
#[derive(Debug, thiserror::Error)]
pub enum CircuitError<E> {
    #[error("Circuit is open - service unavailable")]
    Open,
    #[error("Call failed: {0}")]
    Failed(#[source] E),
}

/// Retry with exponential backoff
pub struct RetryPolicy {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: f64,
}

impl Default for RetryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(10),
            multiplier: 2.0,
        }
    }
}

impl RetryPolicy {
    /// Execute with retry and exponential backoff
    pub async fn execute<F, Fut, T, E>(&self, mut f: F) -> Result<T, E>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Debug,
    {
        let mut delay = self.initial_delay;
        let mut attempts = 0;

        loop {
            match f().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempts += 1;
                    if attempts >= self.max_retries {
                        tracing::error!(
                            attempts = attempts,
                            error = ?e,
                            "Retry exhausted"
                        );
                        return Err(e);
                    }

                    tracing::warn!(
                        attempt = attempts,
                        delay_ms = delay.as_millis(),
                        error = ?e,
                        "Retrying after failure"
                    );

                    // Add jitter (±10%)
                    let jitter = delay.as_millis() as f64 * 0.1;
                    let jittered = delay.as_millis() as f64 
                        + (rand::random::<f64>() * 2.0 - 1.0) * jitter;
                    
                    tokio::time::sleep(Duration::from_millis(jittered as u64)).await;

                    // Exponential backoff
                    delay = Duration::from_millis(
                        (delay.as_millis() as f64 * self.multiplier) as u64
                    ).min(self.max_delay);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_trips() {
        let config = CircuitConfig {
            failure_threshold: 2,
            success_threshold: 1,
            half_open_failure_threshold: 1,
            reset_timeout: Duration::from_millis(100),
            window_duration: Duration::from_secs(60),
        };
        let cb = CircuitBreaker::new("test", config);

        // Should be closed initially
        assert_eq!(cb.state().await, CircuitState::Closed);
        assert!(cb.allow().await);

        // Record failures
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
        cb.record_failure().await;
        assert_eq!(cb.state().await, CircuitState::Open);

        // Should reject in open state
        assert!(!cb.allow().await);

        // Wait for reset timeout
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should transition to half-open
        assert!(cb.allow().await);
        assert_eq!(cb.state().await, CircuitState::HalfOpen);

        // Success should close it
        cb.record_success().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_retry_policy() {
        let policy = RetryPolicy {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            multiplier: 2.0,
        };

        let mut attempts = 0;
        let result: Result<i32, &str> = policy.execute(|| {
            attempts += 1;
            async move {
                if attempts < 3 {
                    Err("failed")
                } else {
                    Ok(42)
                }
            }
        }).await;

        assert_eq!(result, Ok(42));
        assert_eq!(attempts, 3);
    }
}
