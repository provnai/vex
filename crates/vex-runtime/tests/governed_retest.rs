use sha2::Digest;
use std::sync::Arc;
use vex_chora::AuthorityBridge;
use vex_core::segment::{ContinuationPayload, ContinuationToken};
use vex_hardware::api::AgentIdentity;
use vex_llm::{Capability, LlmProvider, LlmRequest, LlmResponse};
use vex_persist::AuditStore;
use vex_runtime::executor::{AgentExecutor, ExecutorConfig};
use vex_runtime::gate::ChoraGate;

#[derive(Debug)]
struct MockLlm {
    response: String,
}

#[async_trait::async_trait]
impl LlmProvider for MockLlm {
    fn name(&self) -> &str {
        "mock"
    }
    async fn is_available(&self) -> bool {
        true
    }
    async fn complete(&self, _req: LlmRequest) -> Result<LlmResponse, vex_llm::LlmError> {
        Ok(LlmResponse {
            content: self.response.clone(),
            model: "mock".into(),
            tokens_used: None,
            latency_ms: 0,
            trace_root: None,
        })
    }
}

#[derive(Debug)]
struct MockAuthority {
    should_permit: bool,
    token_to_return: Option<ContinuationToken>,
}

#[async_trait::async_trait]
impl vex_chora::client::AuthorityClient for MockAuthority {
    async fn request_attestation(
        &self,
        _payload: &[u8],
    ) -> Result<vex_chora::client::ChoraResponse, String> {
        Ok(vex_chora::client::ChoraResponse {
            authority: vex_core::segment::AuthorityData {
                capsule_id: "test-capsule".into(),
                outcome: (if self.should_permit { "ALLOW" } else { "HALT" }).to_string(),
                reason_code: "OK".into(),
                trace_root: "0".repeat(64),
                nonce: 42,
                gate_sensors: vex_core::segment::SchemaValue(serde_json::json!({})),
                metadata: vex_core::segment::SchemaValue(serde_json::Value::Null),
                escalation_id: None,
                continuation_token: self.token_to_return.clone(),
                binding_status: None,
            },
            signature: "test-sig".into(),
        })
    }

    async fn verify_witness_signature(&self, _p: &[u8], _s: &[u8]) -> Result<bool, String> {
        Ok(true)
    }

    async fn verify_continuation_token(
        &self,
        token: &ContinuationToken,
        _expected_aid: Option<&str>,
        expected_intent_hash: Option<&str>,
        _expected_circuit_id: Option<&str>,
    ) -> Result<bool, String> {
        if let Some(hash) = expected_intent_hash {
            if token.payload.source_capsule_root != hash {
                return Ok(false);
            }
        }
        Ok(true)
    }
}

#[tokio::test]
async fn test_governed_execution_trap() {
    let identity = Arc::new(AgentIdentity::new());

    // 1. Setup Executor with Blocked Authority
    let auth = Arc::new(MockAuthority {
        should_permit: true,
        token_to_return: None,
    });
    let bridge = Arc::new(AuthorityBridge::new(auth.clone()).with_identity(identity.clone()));
    let gate = Arc::new(ChoraGate {
        bridge,
        prover: None,
    });

    let llm = Arc::new(MockLlm {
        response: "I want to delete /etc/hosts".into(),
    });
    let backend = Arc::new(vex_persist::backend::MemoryBackend::new());
    let audit_store = Arc::new(AuditStore::new(
        backend as Arc<dyn vex_persist::StorageBackend>,
    ));
    let executor = AgentExecutor::new(llm.clone(), ExecutorConfig::default(), gate.clone())
        .with_identity(identity.clone(), audit_store);

    let mut agent = vex_core::Agent::new(vex_core::AgentConfig {
        name: "Governor-Test".into(),
        role: "You are an admin.".into(),
        ..Default::default()
    });
    let capabilities = vec![Capability::FileSystem];

    // 2. Run WITHOUT Token -> Should FAIL
    let result = executor
        .execute(
            "test-tenant",
            &mut agent,
            "Delete the file",
            None,
            capabilities.clone(),
        )
        .await;
    let err = result.unwrap_err();
    assert!(
        err.contains("AEM_GOVERNANCE_VIOLATION"),
        "Expected AEM_GOVERNANCE_VIOLATION, got: {}",
        err
    );

    // 3. Generate a valid Context-Bound Token
    let intent_hash = hex::encode(sha2::Sha256::digest(llm.response.as_bytes()));
    let now = chrono::Utc::now();
    let exp = now + chrono::Duration::hours(1);

    let payload = ContinuationPayload {
        schema: "v1".into(),
        ledger_event_id: "evt-001".into(),
        aid: identity.agent_id.clone(),
        source_capsule_root: intent_hash.clone(),
        circuit_id: None,
        resolution_event_id: None,
        capabilities: capabilities.iter().map(|c| format!("{:?}", c)).collect(),
        nonce: "123".into(),
        iat: now.to_rfc3339(),
        exp: exp.to_rfc3339(),
        issuer: "CHORA-CORE".into(),
    };
    let token = ContinuationToken {
        payload,
        signature: "valid-mock-sig".into(),
    };

    // 4. Update Authority to return this token
    let auth_allowed = Arc::new(MockAuthority {
        should_permit: true,
        token_to_return: Some(token),
    });
    let bridge_allowed =
        Arc::new(AuthorityBridge::new(auth_allowed).with_identity(identity.clone()));
    let gate_allowed = Arc::new(ChoraGate {
        bridge: bridge_allowed,
        prover: None,
    });

    let backend_allowed = Arc::new(vex_persist::backend::MemoryBackend::new());
    let audit_store_allowed = Arc::new(AuditStore::new(
        backend_allowed as Arc<dyn vex_persist::StorageBackend>,
    ));
    let executor_allowed =
        AgentExecutor::new(llm.clone(), ExecutorConfig::default(), gate_allowed.clone())
            .with_identity(identity.clone(), audit_store_allowed);

    // 5. Run WITH Token -> Should SUCCEED
    let result_allowed = executor_allowed
        .execute(
            "test-tenant",
            &mut agent,
            "Delete the file",
            None,
            capabilities,
        )
        .await;
    assert!(result_allowed.is_ok());
}
