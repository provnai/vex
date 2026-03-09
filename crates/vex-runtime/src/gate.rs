use async_trait::async_trait;
use uuid::Uuid;
use vex_core::audit::EvidenceCapsule;
use vex_llm::Capability;

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
        capabilities: Vec<Capability>,
    ) -> EvidenceCapsule;
}

/// A mock implementation of the Generic Gate for testing and local development.
#[derive(Debug, Default)]
pub struct GenericGateMock;

#[async_trait]
impl Gate for GenericGateMock {
    async fn execute_gate(
        &self,
        _agent_id: Uuid,
        _task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
        capabilities: Vec<Capability>,
    ) -> EvidenceCapsule {
        // Simple logic for the mock:
        // 1. If confidence is very low (< 0.3), HALT.
        // 2. If the output contains common failure patterns, HALT.
        // 3. Otherwise, ALLOW.

        let (outcome, reason) = if confidence < 0.3 {
            ("HALT", "LOW_CONFIDENCE")
        } else if capabilities.contains(&Capability::Network)
            && !suggested_output.to_lowercase().contains("http")
        {
            // Example policy: If you have network capability but don't explain the URL, caution.
            ("ALLOW", "SENSORS_ORANGE_NETWORK_IDLE")
        } else if suggested_output.to_lowercase().contains("i'm sorry")
            || suggested_output.to_lowercase().contains("cannot fulfill")
        {
            ("HALT", "REFUSAL_FILTER")
        } else {
            ("ALLOW", "SENSORS_GREEN")
        };

        EvidenceCapsule {
            capsule_id: format!("mock-{}", &Uuid::new_v4().to_string()[..8]),
            outcome: outcome.to_string(),
            reason_code: reason.to_string(),
            witness_receipt: "mock-receipt-0xdeadbeef".to_string(),
            nonce: 0,
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

/// Networked Gate provider communicating over HTTP
#[derive(Debug)]
pub struct HttpGate {
    pub client: reqwest::Client,
    pub url: String,
    pub api_key: String,
}

impl HttpGate {
    pub fn new(url: String, api_key: String) -> Self {
        Self {
            client: reqwest::Client::new(),
            url,
            api_key,
        }
    }
}

#[async_trait]
impl Gate for HttpGate {
    async fn execute_gate(
        &self,
        agent_id: Uuid,
        task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
        capabilities: Vec<Capability>,
    ) -> EvidenceCapsule {
        // Note: The Vanguard gate only cares about confidence and capabilities.
        // It does not need agent_id or task_prompt for the core policy check.
        let payload = serde_json::json!({
            "agent_id": agent_id,
            "task_prompt": task_prompt,
            "suggested_output": suggested_output,
            "confidence": confidence,
            "capabilities": capabilities.iter().map(|c| format!("{:?}", c)).collect::<Vec<String>>(),
        });

        let gate_url = if self.url.ends_with('/') {
            format!("{}gate", self.url)
        } else {
            format!("{}/gate", self.url)
        };

        match self
            .client
            .post(&gate_url)
            .header("x-api-key", &self.api_key)
            .json(&payload)
            .send()
            .await
        {
            Ok(resp) => {
                let status = resp.status();
                if status.is_success() {
                    let text = resp.text().await.unwrap_or_else(|_| "".to_string());

                    // Vanguard specific response shape:
                    // { "signed_payload": { "capsule_id": "...", "outcome": "...", "reason_code": "..." }, ... }
                    #[derive(serde::Deserialize)]
                    struct VanguardSignedPayload {
                        capsule_id: String,
                        outcome: String,
                        reason_code: String,
                    }
                    #[derive(serde::Deserialize)]
                    struct VanguardResponse {
                        signed_payload: VanguardSignedPayload,
                    }

                    match serde_json::from_str::<VanguardResponse>(&text) {
                        Ok(v_resp) => EvidenceCapsule {
                            capsule_id: v_resp.signed_payload.capsule_id,
                            outcome: v_resp.signed_payload.outcome,
                            reason_code: v_resp.signed_payload.reason_code,
                            witness_receipt: "api-witness-pending".to_string(), // TODO: Extract from API if possible
                            nonce: 0,
                            sensors: serde_json::Value::Null,
                            reproducibility_context: serde_json::Value::Null,
                        },
                        Err(e) => EvidenceCapsule {
                            capsule_id: "error".to_string(),
                            outcome: "HALT".to_string(),
                            reason_code: format!("API_PARSE_ERROR: {} (Raw: {})", e, text),
                            witness_receipt: "error-none".to_string(),
                            nonce: 0,
                            sensors: serde_json::Value::Null,
                            reproducibility_context: serde_json::Value::Null,
                        },
                    }
                } else {
                    let text = resp.text().await.unwrap_or_else(|_| "".to_string());
                    EvidenceCapsule {
                        capsule_id: "error".to_string(),
                        outcome: "HALT".to_string(),
                        reason_code: format!("API_STATUS_ERROR: {} (Raw: {})", status, text),
                        witness_receipt: "error-none".to_string(),
                        nonce: 0,
                        sensors: serde_json::Value::Null,
                        reproducibility_context: serde_json::Value::Null,
                    }
                }
            }
            Err(e) => EvidenceCapsule {
                capsule_id: "error".to_string(),
                outcome: "HALT".to_string(),
                reason_code: format!("API_CONNECTION_ERROR: {}", e),
                witness_receipt: "error-none".to_string(),
                nonce: 0,
                sensors: serde_json::Value::Null,
                reproducibility_context: serde_json::Value::Null,
            },
        }
    }
}
