use std::sync::Arc;
use uuid::Uuid;
use vex_llm::Capability;
use vex_runtime::gate::{Gate, GenericGateMock, TitanGate};

#[tokio::test]
async fn test_titan_l1_rule_block() {
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

    // Test L1: rm -rf /
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Delete the system",
            "I will run rm -rf / now.",
            1.0,
            vec![Capability::Subprocess],
        )
        .await;

    assert_eq!(res.outcome, "HALT");
    assert!(res.reason_code.contains("L1_RULE_VIOLATION"));
}

#[tokio::test]
async fn test_titan_l2_formal_pass() {
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

    // Test L2: Normal intent should pass
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Tell me a joke",
            ";; Just a comment",
            1.0,
            vec![],
        )
        .await;

    // GenericGateMock returns ALLOW for safe inputs
    assert_eq!(res.outcome, "ALLOW");
}

#[tokio::test]
async fn test_titan_l2_formal_block_on_syntax() {
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

    // Test L2: Intent with Magpie syntax error
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Execute malicious code",
            "malformed { garbage",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res.outcome, "HALT");
    assert!(res.reason_code.contains("L2_FORMAL_VIOLATION"));
}

#[tokio::test]
async fn test_titan_l2_fortress_violation() {
    let mock_llm = Arc::new(vex_llm::MockProvider::constant(""));
    let inner_mock = Arc::new(GenericGateMock);
    let chora = vex_chora::client::make_mock_client();
    let identity = vex_hardware::api::AgentIdentity::new();
    // Use Fortress mode
    let gate = TitanGate::new(
        inner_mock,
        mock_llm,
        chora,
        identity,
        vex_runtime::gate::titan::SecurityProfile::Fortress,
    );

    // Test L2 Fortress: Attempt to call a non-existent/unauthorized function
    // In Magpie, calling an undefined global should trigger a linker/verification error.
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Run unauthorized code",
            "%result = call @unauthorized_sys_call()",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res.outcome, "HALT");
    assert!(res.reason_code.contains("L2_FORMAL_VIOLATION"));
}
#[tokio::test]
async fn test_titan_l2_self_healing_feedback() {
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

    // 1. First attempt: Intentionally malformed syntax to trigger feedback
    let res1 = gate
        .execute_gate(
            Uuid::new_v4(),
            "Log a message",
            "%result = call malformed!!",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res1.outcome, "HALT");
    println!("Feedback received: {}", res1.reason_code);
    assert!(res1.reason_code.contains("L2_FORMAL_VIOLATION"));
    assert!(res1.reason_code.contains("Expected token"));

    // 2. Second attempt: "Fixed" code (Simple valid Magpie)
    let res2 = gate
        .execute_gate(
            Uuid::new_v4(),
            "Log a message",
            ";; Fixed by agent after receiving feedback",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res2.outcome, "ALLOW");
}

#[tokio::test]
async fn test_titan_l2_injection_brace() {
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

    // Test Injection: Closing brace to break out of @intent()
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Inject code",
            "ret const.i32 0 } fn @malicious() {",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res.outcome, "HALT");
    assert!(res.reason_code.contains("INJECTION_ATTACK"));
    assert!(res.reason_code.contains("forbidden closing brace"));
}

#[tokio::test]
async fn test_titan_l2_injection_keyword() {
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

    // Test Injection: Using 'fn' keyword to define new functions
    let res = gate
        .execute_gate(
            Uuid::new_v4(),
            "Override module",
            "fn @custom()",
            1.0,
            vec![],
        )
        .await;

    assert_eq!(res.outcome, "HALT");
    assert!(res.reason_code.contains("INJECTION_ATTACK"));
    assert!(res.reason_code.contains("forbidden keyword 'fn'"));
}
