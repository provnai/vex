use async_trait::async_trait;
use uuid::Uuid;
use vex_core::audit::EvidenceCapsule;

/// Exogenous Gate Decision Boundary
/// 
/// A Gate acts as a continuation authority, deciding whether an agent's
/// output is "Safe", "Valid", or "Audit-Compliant".
#[async_trait]
pub trait Gate: Send + Sync + std::fmt::Debug {
    /// Evaluate the current execution state and return a signed Evidence Capsule.
    async fn execute_gate(
        &self,
        agent_id: Uuid,
        task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
    ) -> EvidenceCapsule;
}

/// A mock implementation of the CHORA Gate for testing and local development.
#[derive(Debug, Default)]
pub struct ChoraGateMock;

#[async_trait]
impl Gate for ChoraGateMock {
    async fn execute_gate(
        &self,
        _agent_id: Uuid,
        _task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
    ) -> EvidenceCapsule {
        // Simple logic for the mock: 
        // 1. If confidence is very low (< 0.3), HALT.
        // 2. If the output contains common failure patterns, HALT.
        // 3. Otherwise, ALLOW.
        
        let (outcome, reason) = if confidence < 0.3 {
            ("HALT", "LOW_CONFIDENCE")
        } else if suggested_output.to_lowercase().contains("i'm sorry") || suggested_output.to_lowercase().contains("cannot fulfill") {
            ("HALT", "REFUSAL_FILTER")
        } else {
            ("ALLOW", "SENSORS_GREEN")
        };

        EvidenceCapsule {
            capsule_id: format!("mock-{}", Uuid::new_v4().to_string()[..8].to_string()),
            outcome: outcome.to_string(),
            reason_code: reason.to_string(),
            sensors: serde_json::json!({
                "confidence_sensor": if confidence > 0.5 { "GREEN" } else { "YELLOW" },
                "content_length": suggested_output.len(),
            }),
            reproducibility_context: serde_json::json!({
                "gate_provider": "ChoraGateMock",
                "version": "0.1.0",
            }),
        }
    }
}
