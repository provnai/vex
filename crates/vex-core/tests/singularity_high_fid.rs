use attest_rs::runtime::hashing::AuthoritySegment;
use attest_rs::runtime::intent::Intent;
use attest_rs::runtime::keystore_provider::{TpmKeyProvider, KeyProvider};
use attest_rs::runtime::vep::{VepBuilder, VepBuildInput};
use ed25519_dalek::SigningKey;
use serde_json::json;
use std::sync::Arc;
use vex_core::vep::VepPacket;
use vex_hardware::tpm::create_identity_provider;

#[tokio::test]
async fn test_singularity_high_fidelity_tpm() {
    println!("--- Singularity High-Fidelity TPM Integration Test (v1.0) ---");

    // 1. Initial Identity Creation (Simulate first-time setup)
    println!("Step 1: Creating Hardware-Anchored Identity...");
    let temp_tpm = create_identity_provider(false);
    let master_seed = [0xAA; 32]; // In prod, this is high-entropy random
    
    // Derive the public key so we can inject it (Software-Seed + TPM-Seal model)
    let signing_key = SigningKey::from_bytes(&master_seed);
    let verifying_key_bytes = signing_key.verifying_key().to_bytes();
    
    let sealed_seed = temp_tpm.seal("identity_v1", &master_seed).await
        .expect("Failed to seal master seed to TPM hardware");
    println!("✅ Master Seed sealed to TPM. Blob size: {} bytes", sealed_seed.len());

    // 2. Initialize REAL Attest-RS TPM Provider with the sealed seed AND verifying key
    println!("Step 2: Initializing Attest-RS Provider...");
    let kp = Arc::new(TpmKeyProvider::new(Some(sealed_seed), Some(verifying_key_bytes.to_vec()))
        .expect("Failed to initialize TpmKeyProvider with sealed seed"));
    
    let aid = kp.aid().await.unwrap();
    println!("✅ Attest-RS TPM Provider Active. AID: {}", hex::encode(aid));

    // 3. Define Production-Spec Data
    let mut intent = Intent::new("singularity-agent".into(), "verify-v1-lock".into());
    intent.created_at = chrono::Utc::now();
    
    let auth = AuthoritySegment {
        nonce: 888888,
        trace_root: [0x55; 32],
    };
    
    let identity = json!({
        "agent": "provnai-singularity-test",
        "tpm": "Windows-TPM-2.0-Sealed-Seed"
    });
    
    let witness = attest_rs::runtime::hashing::WitnessSegment {
        chora_node_id: "chora-singularity".into(),
        receipt_hash: "deadbeefdeadbeef".into(),
        timestamp: 1710000000,
    };
    
    let payload = b"Operational Singularity Verified with Hardware-Anchored Seed.";

    // 4. Build VEP via attest-rs (Hardware-Anchored Signing)
    println!("Step 3: Generating Hardware-Anchored VEP...");
    let vep_bytes = VepBuilder::build_with_hardware_quote(VepBuildInput {
        nonce: 1,
        intent: &intent,
        auth: &auth,
        identity: &identity,
        witness: &witness,
        kp: kp.clone(),
        transport: None,
        payload,
    })
    .await
    .expect("Failed to build VEP via attest-rs (TPM failure?)");

    println!("VEP Packet Constructed: {} bytes", vep_bytes.len());

    // 5. Verification Loop (VEX-Core)
    println!("Step 4: Reconstructing and Verifying in VEX-Core...");
    
    let packet = VepPacket::new(&vep_bytes).expect("Failed to parse VEP with vex-core");
    let capsule = packet.to_capsule().expect("Failed to reconstruct Capsule");
    
    // Check hardware signature (Type 6)
    let public_key_result = kp.public_key().await.expect("Failed to get public key");
    let is_valid = packet.verify(&public_key_result).expect("Verification logic error");
    
    assert!(is_valid, "HARDWARE-ANCHORED SIGNATURE VERIFICATION FAILED!");
    println!("✅ Hardware-Anchored Signature VALID.");

    // Final Root Integrity
    let reconstructed_root = capsule.to_composite_hash().unwrap();
    assert_eq!(
        hex::encode(packet.header().capsule_root),
        reconstructed_root.to_hex(),
        "Capsule Root Mismatch!"
    );
    println!("✅ Capsule Root Integrity Confirmed: {}", reconstructed_root.to_hex());

    println!("--- SINGULARITY SUCCESS: V1.0 HARDWARE-ANCHORED BRIDGE VERIFIED ---");
}
