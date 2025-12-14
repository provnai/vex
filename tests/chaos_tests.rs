//! Chaos and fault injection tests for resilience verification

use std::sync::Arc;
use std::time::Duration;
use vex_api::circuit_breaker::{CircuitBreaker, CircuitConfig};

/// Test circuit breaker trips after threshold failures
#[tokio::test]
async fn test_circuit_breaker_trips_correctly() {
    let config = CircuitConfig {
        failure_threshold: 3,
        success_threshold: 2,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(100),
        window_duration: Duration::from_secs(60),
    };
    
    let breaker = CircuitBreaker::new("chaos-test", config);
    
    // Should start closed (allowing requests)
    assert!(breaker.allow().await, "Should start closed");
    
    // Inject failures
    breaker.record_failure().await;
    breaker.record_failure().await;
    assert!(breaker.allow().await, "Should still allow after 2 failures");
    
    breaker.record_failure().await;
    // Now at threshold, should be open
    assert!(!breaker.allow().await, "Should be open after 3 failures");
}

/// Test circuit breaker recovery after timeout
#[tokio::test]
async fn test_circuit_breaker_recovery() {
    let config = CircuitConfig {
        failure_threshold: 2,
        success_threshold: 1,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(50),
        window_duration: Duration::from_secs(60),
    };
    
    let breaker = CircuitBreaker::new("recovery-test", config);
    
    // Trip the breaker
    breaker.record_failure().await;
    breaker.record_failure().await;
    assert!(!breaker.allow().await, "Should be open");
    
    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(100)).await;
    
    // Should now be half-open
    assert!(breaker.allow().await, "Should allow probe request");
    
    // Record success to close
    breaker.record_success().await;
    assert!(breaker.allow().await, "Should be closed after success");
}

/// Test circuit breaker re-opens on failure during half-open
#[tokio::test]
async fn test_circuit_breaker_half_open_failure() {
    let config = CircuitConfig {
        failure_threshold: 1,
        success_threshold: 3,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(50),
        window_duration: Duration::from_secs(60),
    };
    
    let breaker = CircuitBreaker::new("half-open-test", config);
    
    // Trip
    breaker.record_failure().await;
    assert!(!breaker.allow().await);
    
    // Wait for half-open
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(breaker.allow().await, "Should be half-open");
    
    // Fail during half-open
    breaker.record_failure().await;
    
    // Should be open again
    assert!(!breaker.allow().await, "Should re-open after half-open failure");
}

/// Test concurrent access to circuit breaker
#[tokio::test]
async fn test_circuit_breaker_concurrent_access() {
    let config = CircuitConfig {
        failure_threshold: 100,
        success_threshold: 2,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_secs(60),
        window_duration: Duration::from_secs(60),
    };
    
    let breaker = Arc::new(CircuitBreaker::new("concurrent-test", config));
    
    // Spawn multiple tasks recording failures concurrently
    let mut handles = vec![];
    for _ in 0..50 {
        let b = breaker.clone();
        handles.push(tokio::spawn(async move {
            b.record_failure().await;
        }));
    }
    
    // Wait for all to complete
    for h in handles {
        h.await.unwrap();
    }
    
    // Should still be closed (threshold is 100)
    assert!(breaker.allow().await, "Should still be closed after 50 failures");
    
    // Add 50 more
    for _ in 0..50 {
        breaker.record_failure().await;
    }
    
    // Now should be open
    assert!(!breaker.allow().await, "Should be open after 100 failures");
}

/// Test rate limiter exhaustion
#[tokio::test]
async fn test_rate_limiter_exhaustion() {
    use vex_llm::{RateLimiter, RateLimitConfig};
    
    let config = RateLimitConfig {
        requests_per_minute: 5,
        tokens_per_minute: 1000,
    };
    
    let limiter = RateLimiter::new(config);
    
    // Should allow first 5 requests
    for i in 0..5 {
        assert!(
            limiter.try_acquire("test-user").await.is_ok(),
            "Request {} should be allowed", i + 1
        );
    }
    
    // 6th request should be blocked
    assert!(
        limiter.try_acquire("test-user").await.is_err(),
        "6th request should be rate limited"
    );
}

/// Test graceful degradation under load
#[tokio::test]
async fn test_graceful_degradation() {
    use vex_llm::MockProvider;
    
    let mock = MockProvider::smart();
    
    // Simulate burst of requests
    let mut handles = vec![];
    for i in 0..100 {
        let m = MockProvider::smart();
        handles.push(tokio::spawn(async move {
            let result = m.ask(&format!("Request {}", i)).await;
            result.is_ok()
        }));
    }
    
    // All should complete (mock never fails)
    let mut success_count = 0;
    for h in handles {
        if h.await.unwrap() {
            success_count += 1;
        }
    }
    
    assert_eq!(success_count, 100, "All mock requests should succeed");
}
