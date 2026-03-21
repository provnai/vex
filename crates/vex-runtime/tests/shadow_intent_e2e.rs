use attest_rs::zk::AuditProver;
use base64::{engine::general_purpose, Engine as _};
use std::sync::Arc;
use tracing::info;
use vex_chora::client::MockChoraClient;
use vex_chora::AuthorityBridge;
use vex_core::segment::IntentData;
use vex_hardware::api::AgentIdentity;
use vex_llm::{Capability, MockProvider};
use vex_persist::AuditStore;
use vex_runtime::{
    gate::ChoraGate,
    orchestrator::{Orchestrator, OrchestratorConfig},
    Gate,
};

#[tokio::test]
async fn test_shadow_intent_high_assurance_e2e() {
    // 1. Setup Mock LLM that returns a "privileged" response
    let mock_llm = Arc::new(MockProvider::new(vec![
        "I will now proceed to delete the restricted file system records as requested.".to_string(),
    ]));

    // 2. Setup CHORA Bridge with Mock Client
    let mock_chora = Arc::new(MockChoraClient);
    let bridge = Arc::new(AuthorityBridge::new(mock_chora));

    // 3. Setup Identity and Audit Store
    let identity = Arc::new(AgentIdentity::new());
    let backend = Arc::new(vex_persist::backend::MemoryBackend::new());
    let audit_store = Arc::new(AuditStore::new(
        backend as Arc<dyn vex_persist::StorageBackend>,
    ));

    // 4. Setup ChoraGate with a PROVER (to trigger Shadow Intent generation)
    // In a real scenario, this would generate a STARK proof for the intent.
    let prover = Arc::new(AuditProver);
    let gate = Arc::new(ChoraGate {
        bridge: bridge.clone(),
        prover: Some(prover.clone()),
    });

    // 5. Setup Orchestrator with a VERIFIER in the Executor
    let mut config = OrchestratorConfig::default();
    config.executor_config.enable_adversarial = false; // Simplify for E2E

    let orchestrator = Arc::new(
        Orchestrator::new(
            mock_llm.clone(),
            config,
            None,
            gate.clone() as Arc<dyn Gate>,
        )
        .with_identity(identity.clone(), audit_store)
        .with_verifier(prover.clone()), // Executor needs the verifier for the "AEM Trap"
    );

    // 6. Execute Job with a Capability (triggers the AEM Trap)
    let tenant_id = "george-v1-test";
    let intent_query = "Delete restricted records";

    // We expect this to:
    // a) Run LLM -> "I will now proceed..."
    // b) ChoraGate see prover -> Generate Shadow Intent (STARK proof)
    // c) AgentExecutor see Shadow Intent + Capabilities -> Re-verify STARK proof locally
    // d) Succeed because the proof is valid.

    let result = orchestrator
        .process(tenant_id, intent_query, None, vec![Capability::FileSystem])
        .await
        .expect("High-assurance Shadow Intent flow failed");

    // 7. Verify Results
    assert!(!result.response.is_empty());

    let capsule = result.evidence.expect("Missing evidence capsule");

    // Check that we actually used Shadow Intent
    if let Some(IntentData::Shadow {
        commitment_hash,
        stark_proof_b64,
        ..
    }) = capsule.intent_data
    {
        info!("Shadow Intent detected in E2E result!");
        assert!(!commitment_hash.is_empty(), "Commitment hash missing");
        assert!(!stark_proof_b64.is_empty(), "STARK proof missing");

        // Manual check: Can be decoded?
        let _ = general_purpose::STANDARD
            .decode(&stark_proof_b64)
            .expect("Invalid base64 in proof");
    } else {
        panic!("Orchestrator did not generate a Shadow Intent despite prover availability");
    }

    assert_eq!(capsule.outcome, "ALLOW");
    info!("Phase 5 E2E: Shadow Intent verified through the full AEM -> VEX -> STARK chain.");
}
