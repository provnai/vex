use std::time::Duration;
use vex_api::circuit_breaker::{CircuitBreaker, CircuitConfig};

#[tokio::test]
async fn test_circuit_trips_on_failures() {
    let config = CircuitConfig {
        failure_threshold: 3,
        success_threshold: 2,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(100),
        window_duration: Duration::from_secs(10),
    };

    let breaker = CircuitBreaker::new("test-circuit", config);

    // Should start closed
    assert!(breaker.allow().await);

    // Inject failures
    breaker.record_failure().await;
    breaker.record_failure().await;
    assert!(breaker.allow().await); // Still closed (2/3)

    breaker.record_failure().await;
    // Now at 3/3, next check should fail (or maybe immediate switch?
    // Implementation says switch happens inside record_failure)

    // The *next* allow() call should return false immediately if it switched to Open.
    // Wait, allow() checks state first.

    // Let's check logic: record_failure updates state to Open if threshold met.
    // allow() returns false if Open and timeout not passed.

    assert!(
        !breaker.allow().await,
        "Circuit should be open after 3 failures"
    );
}

#[tokio::test]
async fn test_circuit_recovers_after_timeout() {
    let config = CircuitConfig {
        failure_threshold: 2,
        success_threshold: 2,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(50),
        window_duration: Duration::from_secs(10),
    };

    let breaker = CircuitBreaker::new("recovery-test", config);

    // Trip it (needs 2 failures)
    breaker.record_failure().await;
    breaker.record_failure().await;
    assert!(!breaker.allow().await);

    // Wait for reset timeout
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Should allow (HalfOpen now)
    assert!(breaker.allow().await, "Should be allowed (HalfOpen)");

    // Heal it
    breaker.record_success().await; // 1/2 success
    breaker.record_success().await; // 2/2 success -> Closed

    // Verify it stays closed
    assert!(breaker.allow().await);
    // Failure count should be reset
    breaker.record_failure().await;
    assert!(
        breaker.allow().await,
        "Should handle 1 failure without tripping (threshold 1, but count resets)"
    );
}

#[tokio::test]
async fn test_half_open_failure_reopens() {
    let config = CircuitConfig {
        failure_threshold: 1,
        success_threshold: 5,
        half_open_failure_threshold: 1,
        reset_timeout: Duration::from_millis(50),
        window_duration: Duration::from_secs(10),
    };

    let breaker = CircuitBreaker::new("relapse-test", config);

    // Trip
    breaker.record_failure().await;
    assert!(!breaker.allow().await);

    // Wait for HalfOpen
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Probe (activates HalfOpen)
    assert!(breaker.allow().await);

    // Fail immediately
    breaker.record_failure().await;

    // Should be Open again
    assert!(
        !breaker.allow().await,
        "Should re-open after failure in HalfOpen"
    );
}
