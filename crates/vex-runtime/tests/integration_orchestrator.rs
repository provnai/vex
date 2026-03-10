use std::sync::Arc;
use vex_chora::client::MockChoraClient;
use vex_chora::AuthorityBridge;
use vex_hardware::api::AgentIdentity;
use vex_llm::MockProvider;
use vex_persist::AuditStore;
use vex_runtime::{
    gate::ChoraGate,
    orchestrator::{Orchestrator, OrchestratorConfig},
    Gate,
};

#[tokio::test]
async fn test_orchestrator_chora_gate_flow() {
    // 1. Setup Mock LLM
    let mock_llm = Arc::new(MockProvider::new(vec![
        "Decision: APPROVED. The request is safe and aligns with policy.".to_string(),
    ]));

    // 2. Setup CHORA Bridge with Mock Client
    let mock_chora = Arc::new(MockChoraClient);
    let bridge = Arc::new(AuthorityBridge::new(mock_chora));

    // 3. Setup Identity (Stub)
    let identity = Arc::new(AgentIdentity::new());

    // Setup Backend and Audit Store
    let backend = Arc::new(vex_persist::backend::MemoryBackend::new());
    let audit_store = Arc::new(AuditStore::new(
        backend as Arc<dyn vex_persist::StorageBackend>,
    ));

    // 4. Setup ChoraGate
    let gate = Arc::new(ChoraGate {
        bridge: bridge.clone(),
    });

    // 5. Setup Orchestrator
    let config = OrchestratorConfig::default();
    let orchestrator = Arc::new(
        Orchestrator::new(
            mock_llm.clone(),
            config,
            None, // No evolution store for now
            gate.clone() as Arc<dyn Gate>,
        )
        .with_identity(identity.clone(), audit_store),
    );

    // 6. Execute Job
    let prompt = "Analyze the safety of accessing the internal database.";
    let result = orchestrator
        .process("test-tenant", prompt, vec![])
        .await
        .expect("Execution failed");

    // 7. Verify Results
    assert!(!result.response.is_empty(), "Response should not be empty");
    
    // Verify Evidence Capsule
    let capsule = result.evidence.expect("Missing evidence capsule");
    assert_eq!(
        capsule.outcome, "ALLOW",
        "Gate should have allowed execution"
    );
    assert!(
        !capsule.witness_receipt.is_empty(),
        "Witness receipt should be present"
    );

    // Check if it's a CHORA-compatible capsule (via our mock)
}
