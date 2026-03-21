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
        intent_data: Option<vex_core::segment::IntentData>,
        confidence: f64,
        capabilities: &[Capability],
    ) -> EvidenceCapsule;

    /// Verify a Continuation Token against the current execution context.
    async fn verify_token(
        &self,
        token: &vex_core::ContinuationToken,
        expected_aid: Option<&str>,
        expected_intent_hash: Option<&str>,
        expected_circuit_id: Option<&str>,
    ) -> Result<bool, String>;
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
        _intent_data: Option<vex_core::segment::IntentData>,
        confidence: f64,
        capabilities: &[Capability],
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
            gate_sensors: vex_core::segment::SchemaValue(serde_json::json!({
                "confidence_sensor": if confidence > 0.5 { "GREEN" } else { "YELLOW" },
                "content_length": suggested_output.len(),
            })),
            reproducibility_context: vex_core::segment::SchemaValue(serde_json::json!({
                "gate_provider": "ChoraGateMock",
                "version": "0.1.0",
            })),
            resolution_vep_hash: None,
            continuation_token: None,
            intent_data: None,
            vep_blob: None,
        }
    }

    async fn verify_token(
        &self,
        _token: &vex_core::ContinuationToken,
        _expected_aid: Option<&str>,
        _expected_intent_hash: Option<&str>,
        _expected_circuit_id: Option<&str>,
    ) -> Result<bool, String> {
        // Mock always returns true
        Ok(true)
    }
}

/// Networked Gate provider communicating over HTTP.
/// Legacy wrapper: Now uses ChoraGate internally for unified handshake logic.
#[derive(Debug, Clone)]
pub struct HttpGate {
    pub inner: std::sync::Arc<ChoraGate>,
}

impl HttpGate {
    pub fn new(client: std::sync::Arc<dyn vex_chora::client::AuthorityClient>) -> Self {
        let bridge = std::sync::Arc::new(vex_chora::AuthorityBridge::new(client));
        Self {
            inner: std::sync::Arc::new(ChoraGate {
                bridge,
                prover: None,
            }),
        }
    }

    /// Attach a ZK Prover for Shadow Intent generation (Phase 4)
    pub fn with_prover(self, prover: std::sync::Arc<attest_rs::zk::AuditProver>) -> Self {
        Self {
            inner: std::sync::Arc::new(ChoraGate {
                bridge: self.inner.bridge.clone(),
                prover: Some(prover),
            }),
        }
    }

    /// Attach a hardware identity to the underlying bridge.
    pub fn with_identity(self, identity: std::sync::Arc<vex_hardware::api::AgentIdentity>) -> Self {
        let bridge = (*self.inner.bridge).clone().with_identity(identity);
        Self {
            inner: std::sync::Arc::new(ChoraGate {
                bridge: std::sync::Arc::new(bridge),
                prover: self.inner.prover.clone(),
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
        intent_data: Option<vex_core::segment::IntentData>,
        confidence: f64,
        capabilities: &[Capability],
    ) -> EvidenceCapsule {
        self.inner
            .execute_gate(
                agent_id,
                task_prompt,
                suggested_output,
                intent_data,
                confidence,
                capabilities,
            )
            .await
    }
    async fn verify_token(
        &self,
        token: &vex_core::ContinuationToken,
        expected_aid: Option<&str>,
        expected_intent_hash: Option<&str>,
        expected_circuit_id: Option<&str>,
    ) -> Result<bool, String> {
        self.inner
            .verify_token(
                token,
                expected_aid,
                expected_intent_hash,
                expected_circuit_id,
            )
            .await
    }
}

/// The default CHORA Gate implementation using the unified AuthorityBridge.
#[derive(Debug, Clone)]
pub struct ChoraGate {
    pub bridge: std::sync::Arc<vex_chora::AuthorityBridge>,
    pub prover: Option<std::sync::Arc<attest_rs::zk::AuditProver>>,
}

#[async_trait]
impl Gate for ChoraGate {
    async fn execute_gate(
        &self,
        _agent_id: Uuid,
        _task_prompt: &str,
        suggested_output: &str,
        intent_data: Option<vex_core::segment::IntentData>,
        confidence: f64,
        capabilities: &[Capability],
    ) -> EvidenceCapsule {
        // 1. Build IntentData (Transparent or Shadow if prover is available)
        // If intent_data was provided by the caller, we use it; otherwise generate it.
        let intent = if let Some(id) = intent_data {
            id
        } else if let Some(_prover) = &self.prover {
            let intent_hash = hex::encode(sha2::Sha256::digest(suggested_output.as_bytes()));

            // Generate STARK Proof for the intent transition [0 -> intent_hash]
            // Note: In Phase 4, we use the core AuditAir for this proof.
            let proof_blob = attest_rs::zk::AuditProver::prove_transition(
                [0u8; 32],
                hex::decode(&intent_hash)
                    .unwrap_or_default()
                    .try_into()
                    .unwrap_or([0u8; 32]),
            )
            .unwrap_or_default();

            vex_core::segment::IntentData::Shadow {
                commitment_hash: intent_hash,
                stark_proof_b64: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    &proof_blob,
                ),
                public_inputs: vex_core::segment::SchemaValue(serde_json::json!({
                    "start_root": "00".repeat(32),
                    "policy": "audit-v1"
                })),
                circuit_id: Some(attest_rs::zk::AuditProver::CIRCUIT_ID.to_string()),
                metadata: vex_core::segment::SchemaValue::default(),
            }
        } else {
            vex_core::segment::IntentData::Transparent {
                request_sha256: hex::encode(sha2::Sha256::digest(suggested_output.as_bytes())),
                confidence,
                capabilities: capabilities.iter().map(|c| format!("{:?}", c)).collect(),
                magpie_source: None,
                metadata: vex_core::segment::SchemaValue::default(),
            }
        };

        // 2. Perform Handshake via Unified Bridge
        match self.bridge.perform_handshake(intent.clone()).await {
            Ok(capsule) => EvidenceCapsule {
                capsule_id: capsule.capsule_id,
                outcome: capsule.authority.outcome,
                reason_code: capsule.authority.reason_code,
                witness_receipt: capsule.witness.receipt_hash,
                nonce: capsule.authority.nonce,
                magpie_source: None,
                gate_sensors: vex_core::segment::SchemaValue(serde_json::json!({
                    "trace_root": capsule.authority.trace_root,
                    "identity_type": capsule.identity.identity_type,
                })),
                reproducibility_context: vex_core::segment::SchemaValue(serde_json::json!({
                    "gate_provider": "ChoraGate",
                    "bridge_version": "v0.2.0",
                })),
                resolution_vep_hash: None,
                continuation_token: capsule.authority.continuation_token,
                intent_data: Some(intent),
                vep_blob: None,
            },
            Err(e) => EvidenceCapsule {
                capsule_id: "error".to_string(),
                outcome: "HALT".to_string(),
                reason_code: format!("CHORA_BRIDGE_ERROR: {}", e),
                witness_receipt: "error-none".to_string(),
                nonce: 0,
                magpie_source: None,
                gate_sensors: vex_core::segment::SchemaValue(serde_json::Value::Null),
                reproducibility_context: vex_core::segment::SchemaValue(serde_json::Value::Null),
                resolution_vep_hash: None,
                continuation_token: None,
                intent_data: None,
                vep_blob: None,
            },
        }
    }

    async fn verify_token(
        &self,
        token: &vex_core::ContinuationToken,
        expected_aid: Option<&str>,
        expected_intent_hash: Option<&str>,
        expected_circuit_id: Option<&str>,
    ) -> Result<bool, String> {
        self.bridge
            .verify_continuation_token(
                token,
                expected_aid,
                expected_intent_hash,
                expected_circuit_id,
            )
            .await
    }
}

pub mod titan;
pub use titan::TitanGate;
