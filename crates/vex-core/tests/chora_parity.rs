use sha2::Digest;
use vex_core::segment::{AuthorityData, Capsule, IdentityData, IntentData, WitnessData};

#[test]
fn test_capsule_jcs_parity() {
    // 1. Construct the native Rust structs
    let intent = IntentData::Transparent {
        request_sha256: "8ee6010d905547c377c67e63559e989b8073b168f11a1ffefd092c7ca962076e"
            .to_string(),
        confidence: 0.95,
        capabilities: vec!["TPM_VERIFY".into()],
        magpie_source: None,
        metadata: serde_json::Value::Null,
    };

    let authority = AuthorityData {
        capsule_id: "chora-v1-test".into(),
        outcome: "ALLOW".into(),
        reason_code: "WITHIN_POLICY".into(),
        trace_root: "5555555555555555555555555555555555555555555555555555555555555555".into(),
        nonce: 12345,
        escalation_id: None,
        binding_status: None,
        continuation_token: None,
        gate_sensors: serde_json::Value::Null,
        metadata: serde_json::Value::Null,
    };

    let identity = IdentityData {
        aid: "test-agent-aid".into(),
        identity_type: "VEX_TPM_v1".into(),
        pcrs: None,
        metadata: serde_json::Value::Null,
    };

    let witness = WitnessData {
        chora_node_id: "test-chora-node".into(),
        receipt_hash: "deadbeef".into(),
        timestamp: 1710396000,
        metadata: serde_json::Value::Null,
    };

    let capsule = Capsule {
        capsule_id: "test-capsule-1".into(),
        intent: intent.clone(),
        authority: authority.clone(),
        identity: identity.clone(),
        witness: witness.clone(),
        intent_hash: "".into(),
        authority_hash: "".into(),
        identity_hash: "".into(),
        witness_hash: "".into(),
        capsule_root: "".into(),
        crypto: vex_core::segment::CryptoData {
            algo: "ed25519".into(),
            public_key_endpoint: "https://auth.provnai.com/keys/test".into(),
            signature_scope: "capsule_root".into(),
            signature_b64: "dGVzdC1zaWduYXR1cmU=".into(),
        },
        request_commitment: None,
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
    let wit_hash = witness.to_commitment_hash().unwrap();

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
