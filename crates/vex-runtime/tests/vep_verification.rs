use vex_runtime::audit::vep::{
    AuthoritySegment, EvidenceCapsuleV0, IdentitySegment, IntentSegment, WitnessSegment, VEP_MAGIC,
    VEP_VERSION,
};

#[test]
fn test_vep_binary_serialization() {
    let intent = IntentSegment {
        request_sha256: "0".repeat(64),
        confidence: 0.95,
        capabilities: vec!["Subprocess".to_string()],
        magpie_source: None,
        metadata: serde_json::Value::Null,
    };

    let authority = AuthoritySegment {
        capsule_id: "test-capsule".to_string(),
        outcome: "ALLOW".to_string(),
        reason_code: "OK".to_string(),
        trace_root: "0".repeat(64),
        nonce: 12345,
        gate_sensors: serde_json::Value::Null,
        metadata: serde_json::Value::Null,
    };

    let identity = IdentitySegment {
        aid: "1".repeat(64),
        identity_type: "TPM_ECC".to_string(),
        pcrs: None,
        metadata: serde_json::Value::Null,
    };

    let witness = WitnessSegment {
        chora_node_id: "node-1".to_string(),
        receipt_hash: "2".repeat(64),
        timestamp: 1710396000,
        metadata: serde_json::json!({}),
    };

    let capsule = EvidenceCapsuleV0::new(intent, authority, identity, witness, None).unwrap();
    let binary = capsule.to_vep_binary().unwrap();

    // Header: magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8) = 76 bytes
    assert!(binary.len() >= 76);
    assert_eq!(&binary[0..3], &VEP_MAGIC);
    assert_eq!(binary[3], VEP_VERSION);

    // Check AID (32 bytes from "11...11")
    let aid_bytes = hex::decode("1".repeat(64)).unwrap();
    assert_eq!(&binary[4..36], &aid_bytes);

    // Check Root (32 bytes)
    let root_bytes = hex::decode(&capsule.capsule_root).unwrap();
    assert_eq!(&binary[36..68], &root_bytes);

    // Check Nonce (8 bytes Big Endian)
    assert_eq!(&binary[68..76], &12345u64.to_be_bytes());
}

#[test]
fn test_vep_jcs_parity() {
    // Reference parity vector from spec v0.1:
    // Intent Hash: e02504ea...
    // Authority Hash: 6fac0de3...
    // Identity Hash: 7869bae0...
    // Witness Hash: 174dfb80...
    // Definitive Capsule Root: 71d0324716f378b724e6186340289ecad5b99d6301d1585a322f2518db52693e

    // We need to ensure Binary Merkle Tree construction matches the v0.3 spec
    use vex_core::merkle::{Hash, MerkleTree};
    
    let intent_h = Hash::from_bytes(hex::decode("e02504ea88bd9f05a744cd8a462a114dc2045eb7210ea8c6f5ff2679663c92cb").unwrap().try_into().unwrap());
    let authority_h = Hash::from_bytes(hex::decode("6fac0de31355fc1dfe36eee1e0c226f7cc36dd58eaad0aca0c2d3873b4784d35").unwrap().try_into().unwrap());
    let identity_h = Hash::from_bytes(hex::decode("7869bae0249b33e09b881a0b44faba6ee3f4bab7edcc2aa5a5e9290e2563c828").unwrap().try_into().unwrap());
    let witness_h = Hash::from_bytes(hex::decode("174dfb80917cca8a8d4760b82656e78df0778cb3aadd60b51cd018b3313d5733").unwrap().try_into().unwrap());

    let leaves = vec![
        ("intent".to_string(), intent_h),
        ("authority".to_string(), authority_h),
        ("identity".to_string(), identity_h),
        ("witness".to_string(), witness_h),
    ];

    let tree = MerkleTree::from_leaves(leaves);
    let root_hash = tree.root_hash().unwrap().to_hex();

    // The Merkle root for these specific hashes (v0.3)
    assert_eq!(
        root_hash,
        "dc7be62bcb8705ba383b518999fb191dca9220aaa9d8c2a5b9070f5aa5686ad1"
    );
}

#[test]
fn test_vep_signature_verification() {
    use base64::Engine as _;
    use ed25519_dalek::{Signature, SigningKey, Verifier, VerifyingKey};

    let mut capsule = EvidenceCapsuleV0::new(
        IntentSegment {
            request_sha256: "0".repeat(64),
            confidence: 1.0,
            capabilities: vec![],
            magpie_source: None,
            metadata: serde_json::Value::Null,
        },
        AuthoritySegment {
            capsule_id: "id".into(),
            outcome: "ALLOW".into(),
            reason_code: "OK".into(),
            trace_root: "0".repeat(64),
            nonce: 1,
            gate_sensors: serde_json::Value::Null,
            metadata: serde_json::Value::Null,
        },
        IdentitySegment {
            aid: "1".repeat(64),
            identity_type: "TPM".into(),
            pcrs: None,
            metadata: serde_json::Value::Null,
        },
        WitnessSegment {
            chora_node_id: "n".into(),
            receipt_hash: "2".repeat(64),
            timestamp: 1710396000,
            metadata: serde_json::json!({}),
        },
        None,
    )
    .unwrap();

    let signing_key = SigningKey::from_bytes(&[0u8; 32]);
    let verifying_key: VerifyingKey = (&signing_key).into();

    capsule.sign(&signing_key).unwrap();

    // Verify math
    let root_bytes = hex::decode(&capsule.capsule_root).unwrap();
    let sig_bytes = base64::engine::general_purpose::STANDARD
        .decode(&capsule.crypto.signature_b64)
        .unwrap();
    let sig = Signature::from_bytes(sig_bytes.as_slice().try_into().unwrap());

    assert!(verifying_key.verify(&root_bytes, &sig).is_ok());
}
