use std::sync::Arc;
use uuid::Uuid;
use vex_llm::Capability;
use vex_runtime::audit::verify::VepVerifier;
use vex_runtime::gate::{Gate, GenericGateMock, TitanGate};

#[tokio::test]
async fn test_vep_end_to_end_bundling_and_reverification() {
    let mock_llm = Arc::new(vex_llm::MockProvider::constant(""));
    let inner_mock = Arc::new(GenericGateMock);
    let chora = vex_chora::client::make_mock_client();
    let identity = vex_hardware::api::AgentIdentity::new();
    let gate = TitanGate::new(
        inner_mock,
        mock_llm,
        chora,
        identity,
        vex_runtime::gate::titan::SecurityProfile::Standard,
    );

    // 1. Execute Gate with valid intent
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Generate a report",
            ";; Valid Magpie comment",
            0.95,
            vec![Capability::FileSystem],
        )
        .await;

    assert_eq!(res.outcome, "ALLOW");

    // 2. Verify VEP blob exists and is valid TLV (integrity-only, no signature key available)
    let vep_blob = res.vep_blob.expect("Missing VEP blob in result");

    // Use integrity-only mode since AgentIdentity doesn't expose verifying key
    let capsule = VepVerifier::verify_binary(&vep_blob, None).expect("VEP integrity check failed");

    // 3. Verify Magpie AST is bundled
    let bundled_source = capsule
        .intent
        .magpie_source
        .as_ref()
        .expect("Missing bundled Magpie AST");
    assert!(bundled_source.contains("module intent.verify"));
    assert!(bundled_source.contains("Valid Magpie comment"));

    println!("Full VEP Bundle Verified successfully!");
}

#[tokio::test]
async fn test_vep_bundling_with_sensors() {
    let mock_llm = Arc::new(vex_llm::MockProvider::constant(""));
    let inner_mock = Arc::new(GenericGateMock);
    let chora = vex_chora::client::make_mock_client();
    let identity = vex_hardware::api::AgentIdentity::new();
    let gate = TitanGate::new(
        inner_mock,
        mock_llm,
        chora,
        identity,
        vex_runtime::gate::titan::SecurityProfile::Fortress,
    );

    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Secure task",
            ";; Fortress run",
            1.0,
            vec![],
        )
        .await;

    let vep_blob = res.vep_blob.expect("Missing VEP blob");
    let capsule = VepVerifier::verify_binary(&vep_blob, None).expect("Integrity check failed");

    // Verify sensors captured the profile
    let sensors = capsule.authority.gate_sensors;
    assert_eq!(sensors["profile"], "Fortress");
    assert!(sensors.get("l2_digest").is_some());
}
