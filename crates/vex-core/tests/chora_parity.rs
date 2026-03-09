use sha2::Digest;
use vex_core::segment::{AuthorityData, Capsule, IdentityData, IntentData, WitnessData};

#[test]
fn test_capsule_jcs_parity() {
    // 1. Construct the native Rust structs
    let intent = IntentData {
        id: "test-intent-1".into(),
        goal: "test-goal".into(),
        description: None,
        ticket_id: None,
        constraints: vec![],
        acceptance_criteria: vec![],
        status: "open".into(),
        created_at: "2024-01-01T00:00:00Z".into(),
        closed_at: None,
    };

    let authority = AuthorityData {
        capsule_id: "chora-v1-test".into(),
        outcome: "ALLOW".into(),
        reason_code: "WITHIN_POLICY".into(),
        trace_root: [0x55; 32],
        nonce: 12345,
    };

    let identity = IdentityData {
        agent: "test-agent".into(),
        tpm: "test-tpm".into(),
    };

    let witness = WitnessData {
        chora_node_id: "test-chora-node".into(),
        receipt_hash: "deadbeef".into(),
        timestamp: "2024-03-09T10:00:00Z".into(),
    };

    let capsule = Capsule {
        intent: intent.clone(),
        authority: authority.clone(),
        identity: identity.clone(),
        witness: witness.clone(),
        chora_signature: "".into(),
    };

    // 2. Compute individual pillar hashes to show the intermediate state
    let intent_hash = intent.to_jcs_hash().unwrap();

    // Hash helper for the other structs
    fn hash_seg<T: serde::Serialize>(seg: &T) -> String {
        let jcs = serde_jcs::to_vec(seg).unwrap();
        let mut hasher = sha2::Sha256::new();
        sha2::Digest::update(&mut hasher, &jcs);
        hex::encode(hasher.finalize())
    }

    let auth_hash = hash_seg(&authority);
    let id_hash = hash_seg(&identity);
    let wit_hash = hash_seg(&witness);

    println!("--- RUST NATIVE HASHES ---");
    println!("Intent Hash:    {}", intent_hash.to_hex());
    println!("Authority Hash: {}", auth_hash);
    println!("Identity Hash:  {}", id_hash);
    println!("Witness Hash:   {}", wit_hash);

    // 3. Compute the full composite capsule root
    let root = capsule.to_composite_hash().unwrap();
    println!("Capsule Root:   {}", root.to_hex());

    // This asserts that the process completes without panicking and results in a valid SHA256 length
    assert!(root.to_hex().len() == 64);
}
