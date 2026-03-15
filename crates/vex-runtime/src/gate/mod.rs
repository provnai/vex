use async_trait::async_trait;
use sha2::Digest;
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
            magpie_source: None,
            gate_sensors: serde_json::json!({
                "confidence_sensor": if confidence > 0.5 { "GREEN" } else { "YELLOW" },
                "content_length": suggested_output.len(),
            }),
            reproducibility_context: serde_json::json!({
                "gate_provider": "ChoraGateMock",
                "version": "0.1.0",
            }),
            vep_blob: None,
        }
    }
}

/// Networked Gate provider communicating over HTTP.
/// Legacy wrapper: Now uses ChoraGate internally for unified handshake logic.
#[derive(Debug, Clone)]
pub struct HttpGate {
    pub inner: std::sync::Arc<ChoraGate>,
}

impl HttpGate {
    pub fn new(url: String, api_key: String) -> Self {
        let client = vex_chora::client::make_authority_client(url, api_key);
        let bridge = std::sync::Arc::new(vex_chora::AuthorityBridge::new(client));
        Self {
            inner: std::sync::Arc::new(ChoraGate { bridge }),
        }
    }

    /// Attach a hardware identity to the underlying bridge.
    pub fn with_identity(self, identity: std::sync::Arc<vex_hardware::api::AgentIdentity>) -> Self {
        let bridge = (*self.inner.bridge).clone().with_identity(identity);
        Self {
            inner: std::sync::Arc::new(ChoraGate {
                bridge: std::sync::Arc::new(bridge),
            }),
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
        self.inner
            .execute_gate(
                agent_id,
                task_prompt,
                suggested_output,
                confidence,
                capabilities,
            )
            .await
    }
}

/// The default CHORA Gate implementation using the unified AuthorityBridge.
#[derive(Debug, Clone)]
pub struct ChoraGate {
    pub bridge: std::sync::Arc<vex_chora::AuthorityBridge>,
}

#[async_trait]
impl Gate for ChoraGate {
    async fn execute_gate(
        &self,
        _agent_id: Uuid,
        _task_prompt: &str,
        suggested_output: &str,
        confidence: f64,
        capabilities: Vec<Capability>,
    ) -> EvidenceCapsule {
        // 1. Build IntentData from execution context
        let intent = vex_core::segment::IntentData::Transparent {
            request_sha256: hex::encode(sha2::Sha256::digest(suggested_output.as_bytes())),
            confidence,
            capabilities: capabilities.iter().map(|c| format!("{:?}", c)).collect(),
            magpie_source: None,
            metadata: serde_json::Value::Null,
        };

        // 2. Perform Handshake via Unified Bridge
        match self.bridge.perform_handshake(intent).await {
            Ok(capsule) => EvidenceCapsule {
                capsule_id: capsule.capsule_id,
                outcome: capsule.authority.outcome,
                reason_code: capsule.authority.reason_code,
                witness_receipt: capsule.witness.receipt_hash,
                nonce: capsule.authority.nonce,
                magpie_source: None,
                gate_sensors: serde_json::json!({
                    "trace_root": capsule.authority.trace_root,
                    "identity_type": capsule.identity.identity_type,
                }),
                reproducibility_context: serde_json::json!({
                    "gate_provider": "ChoraGate",
                    "bridge_version": "v0.2.0",
                }),
                vep_blob: None,
            },
            Err(e) => EvidenceCapsule {
                capsule_id: "error".to_string(),
                outcome: "HALT".to_string(),
                reason_code: format!("CHORA_BRIDGE_ERROR: {}", e),
                witness_receipt: "error-none".to_string(),
                nonce: 0,
                magpie_source: None,
                gate_sensors: serde_json::Value::Null,
                reproducibility_context: serde_json::Value::Null,
                vep_blob: None,
            },
        }
    }
}

pub mod titan;
pub use titan::TitanGate;
