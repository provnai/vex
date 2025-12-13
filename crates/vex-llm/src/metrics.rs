//! Metrics and tracing for VEX

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use serde::{Serialize, Deserialize};

/// Global metrics collector
#[derive(Debug, Default)]
pub struct Metrics {
    /// Total LLM calls
    pub llm_calls: AtomicU64,
    /// Total LLM errors
    pub llm_errors: AtomicU64,
    /// Total tokens used
    pub tokens_used: AtomicU64,
    /// Total debates run
    pub debates: AtomicU64,
    /// Total agents created
    pub agents_created: AtomicU64,
    /// Total verifications (adversarial)
    pub verifications: AtomicU64,
    /// Successful verifications
    pub verifications_passed: AtomicU64,
    /// Audit events logged
    pub audit_events: AtomicU64,
}

impl Metrics {
    /// Create new metrics collector
    pub fn new() -> Self {
        Self::default()
    }

    /// Record an LLM call
    pub fn record_llm_call(&self, tokens: u64, error: bool) {
        self.llm_calls.fetch_add(1, Ordering::Relaxed);
        self.tokens_used.fetch_add(tokens, Ordering::Relaxed);
        if error {
            self.llm_errors.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record a debate
    pub fn record_debate(&self) {
        self.debates.fetch_add(1, Ordering::Relaxed);
    }

    /// Record agent creation
    pub fn record_agent_created(&self) {
        self.agents_created.fetch_add(1, Ordering::Relaxed);
    }

    /// Record verification
    pub fn record_verification(&self, passed: bool) {
        self.verifications.fetch_add(1, Ordering::Relaxed);
        if passed {
            self.verifications_passed.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Record audit event
    pub fn record_audit_event(&self) {
        self.audit_events.fetch_add(1, Ordering::Relaxed);
    }

    /// Get snapshot of all metrics
    pub fn snapshot(&self) -> MetricsSnapshot {
        MetricsSnapshot {
            llm_calls: self.llm_calls.load(Ordering::Relaxed),
            llm_errors: self.llm_errors.load(Ordering::Relaxed),
            tokens_used: self.tokens_used.load(Ordering::Relaxed),
            debates: self.debates.load(Ordering::Relaxed),
            agents_created: self.agents_created.load(Ordering::Relaxed),
            verifications: self.verifications.load(Ordering::Relaxed),
            verifications_passed: self.verifications_passed.load(Ordering::Relaxed),
            audit_events: self.audit_events.load(Ordering::Relaxed),
        }
    }

    /// Get verification success rate
    pub fn verification_rate(&self) -> f64 {
        let total = self.verifications.load(Ordering::Relaxed);
        let passed = self.verifications_passed.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            passed as f64 / total as f64
        }
    }

    /// Get LLM error rate
    pub fn llm_error_rate(&self) -> f64 {
        let total = self.llm_calls.load(Ordering::Relaxed);
        let errors = self.llm_errors.load(Ordering::Relaxed);
        if total == 0 {
            0.0
        } else {
            errors as f64 / total as f64
        }
    }
}

/// Snapshot of metrics at a point in time
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsSnapshot {
    pub llm_calls: u64,
    pub llm_errors: u64,
    pub tokens_used: u64,
    pub debates: u64,
    pub agents_created: u64,
    pub verifications: u64,
    pub verifications_passed: u64,
    pub audit_events: u64,
}

impl MetricsSnapshot {
    /// Export metrics in Prometheus text format
    pub fn to_prometheus(&self) -> String {
        let mut output = String::new();
        
        // LLM metrics
        output.push_str("# HELP vex_llm_calls_total Total number of LLM API calls\n");
        output.push_str("# TYPE vex_llm_calls_total counter\n");
        output.push_str(&format!("vex_llm_calls_total {}\n", self.llm_calls));
        
        output.push_str("# HELP vex_llm_errors_total Total number of LLM API errors\n");
        output.push_str("# TYPE vex_llm_errors_total counter\n");
        output.push_str(&format!("vex_llm_errors_total {}\n", self.llm_errors));
        
        output.push_str("# HELP vex_tokens_used_total Total tokens consumed by LLM calls\n");
        output.push_str("# TYPE vex_tokens_used_total counter\n");
        output.push_str(&format!("vex_tokens_used_total {}\n", self.tokens_used));
        
        // Agent metrics
        output.push_str("# HELP vex_agents_created_total Total number of agents created\n");
        output.push_str("# TYPE vex_agents_created_total counter\n");
        output.push_str(&format!("vex_agents_created_total {}\n", self.agents_created));
        
        output.push_str("# HELP vex_debates_total Total number of debates conducted\n");
        output.push_str("# TYPE vex_debates_total counter\n");
        output.push_str(&format!("vex_debates_total {}\n", self.debates));
        
        // Verification metrics
        output.push_str("# HELP vex_verifications_total Total adversarial verifications\n");
        output.push_str("# TYPE vex_verifications_total counter\n");
        output.push_str(&format!("vex_verifications_total {}\n", self.verifications));
        
        output.push_str("# HELP vex_verifications_passed_total Successful verifications\n");
        output.push_str("# TYPE vex_verifications_passed_total counter\n");
        output.push_str(&format!("vex_verifications_passed_total {}\n", self.verifications_passed));
        
        // Audit metrics
        output.push_str("# HELP vex_audit_events_total Total audit events logged\n");
        output.push_str("# TYPE vex_audit_events_total counter\n");
        output.push_str(&format!("vex_audit_events_total {}\n", self.audit_events));
        
        // Derived gauges
        let error_rate = if self.llm_calls > 0 {
            self.llm_errors as f64 / self.llm_calls as f64
        } else {
            0.0
        };
        output.push_str("# HELP vex_llm_error_rate Current LLM error rate\n");
        output.push_str("# TYPE vex_llm_error_rate gauge\n");
        output.push_str(&format!("vex_llm_error_rate {:.4}\n", error_rate));
        
        let verification_rate = if self.verifications > 0 {
            self.verifications_passed as f64 / self.verifications as f64
        } else {
            0.0
        };
        output.push_str("# HELP vex_verification_success_rate Verification success rate\n");
        output.push_str("# TYPE vex_verification_success_rate gauge\n");
        output.push_str(&format!("vex_verification_success_rate {:.4}\n", verification_rate));
        
        output
    }
}

/// Timer for measuring durations
pub struct Timer {
    start: Instant,
    name: String,
}

impl Timer {
    pub fn new(name: &str) -> Self {
        Self {
            start: Instant::now(),
            name: name.to_string(),
        }
    }

    pub fn elapsed(&self) -> Duration {
        self.start.elapsed()
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.elapsed().as_millis() as u64
    }
}

impl Drop for Timer {
    fn drop(&mut self) {
        // In production, this would emit to a tracing system
        #[cfg(debug_assertions)]
        {
            let elapsed = self.elapsed();
            if elapsed > Duration::from_secs(1) {
                eprintln!("[SLOW] {} took {:?}", self.name, elapsed);
            }
        }
    }
}

/// Trace span for structured logging
#[derive(Debug)]
pub struct Span {
    name: String,
    start: Instant,
    attributes: Vec<(String, String)>,
}

impl Span {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            start: Instant::now(),
            attributes: Vec::new(),
        }
    }

    pub fn set_attribute(&mut self, key: &str, value: &str) {
        self.attributes.push((key.to_string(), value.to_string()));
    }

    pub fn with_attribute(mut self, key: &str, value: &str) -> Self {
        self.set_attribute(key, value);
        self
    }
}

impl Drop for Span {
    fn drop(&mut self) {
        // In production, this would emit to OpenTelemetry
        #[cfg(debug_assertions)]
        {
            let elapsed = self.start.elapsed();
            if !self.attributes.is_empty() || elapsed > Duration::from_millis(100) {
                eprintln!(
                    "[TRACE] {} ({:?}) {:?}",
                    self.name, elapsed, self.attributes
                );
            }
        }
    }
}

/// Global metrics instance
static GLOBAL_METRICS: std::sync::OnceLock<Arc<Metrics>> = std::sync::OnceLock::new();

/// Get or initialize global metrics
pub fn global_metrics() -> Arc<Metrics> {
    GLOBAL_METRICS
        .get_or_init(|| Arc::new(Metrics::new()))
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics() {
        let metrics = Metrics::new();

        metrics.record_llm_call(100, false);
        metrics.record_llm_call(50, true);
        metrics.record_verification(true);
        metrics.record_verification(false);

        let snapshot = metrics.snapshot();
        assert_eq!(snapshot.llm_calls, 2);
        assert_eq!(snapshot.llm_errors, 1);
        assert_eq!(snapshot.tokens_used, 150);
        assert_eq!(snapshot.verifications, 2);
        assert_eq!(snapshot.verifications_passed, 1);

        assert_eq!(metrics.verification_rate(), 0.5);
        assert_eq!(metrics.llm_error_rate(), 0.5);
    }

    #[test]
    fn test_timer() {
        let timer = Timer::new("test_operation");
        std::thread::sleep(std::time::Duration::from_millis(10));
        assert!(timer.elapsed_ms() >= 10);
    }
}
