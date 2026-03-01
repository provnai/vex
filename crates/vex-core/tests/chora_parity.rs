use serde_json::json;
use sha2::{Digest, Sha256};
use vex_core::audit::{ActorType, AuditEventType, EvidenceCapsule, HashParams};

#[test]
fn test_chora_parity() {
    let event_type = AuditEventType::GateDecision;
    let actor = ActorType::System("chora_ref".to_string());
    
    let data = json!({
        "probe": "P4_contradiction_pressure",
        "run_id": "20260228_074549__ON"
    });
    
    let rationale = None;
    let policy_version = None;
    let data_provenance_hash = None;
    let human_review_required = false;
    let approval_count = 0;
    
    let evidence_capsule = Some(EvidenceCapsule {
        capsule_id: "cap_anchor_P4_v1".to_string(),
        outcome: "HALT".to_string(),
        reason_code: "RC_CONTRADICTION".to_string(),
        sensors: serde_json::Value::Null,
        reproducibility_context: json!({
            "engine": "deepseek-chat",
            "temperature": "0.2"
        }),
    });
    
    let params = HashParams {
        event_type: &event_type,
        timestamp: 1700000000,
        sequence_number: 1,
        data: &data,
        actor: &actor,
        rationale: &rationale,
        policy_version: &policy_version,
        data_provenance_hash: &data_provenance_hash,
        human_review_required,
        approval_count,
        evidence_capsule: &evidence_capsule,
        schema_version: "1.0",
    };
    
    let jcs_bytes = serde_jcs::to_vec(&params).expect("Failed to serialize JCS");
    let jcs_string = String::from_utf8(jcs_bytes.clone()).unwrap();
    
    let target_jcs = r#"{"actor":{"id":"chora_ref","type":"system"},"approval_count":0,"data":{"probe":"P4_contradiction_pressure","run_id":"20260228_074549__ON"},"event_type":"CHORA_GATE_DECISION","evidence_capsule":{"capsule_id":"cap_anchor_P4_v1","outcome":"HALT","reason_code":"RC_CONTRADICTION","reproducibility_context":{"engine":"deepseek-chat","temperature":"0.2"}},"human_review_required":false,"schema_version":"1.0","sequence_number":1,"timestamp":1700000000}"#;
    
    std::fs::write("jcs_output.txt", jcs_string.clone()).unwrap();
    std::fs::write("jcs_target.txt", target_jcs.to_string()).unwrap();
    
    let mut hasher = Sha256::new();
    hasher.update(&jcs_bytes);
    let hash_hex = hex::encode(hasher.finalize());
    println!("Hash is: {}", hash_hex);
}
