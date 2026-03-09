use attest_rs::runtime::hashing::{AuthoritySegment, WitnessSegment as AttestWitnessSegment};
use attest_rs::runtime::intent::Intent;
use attest_rs::runtime::noise::MockKeyProvider;
use attest_rs::runtime::vep::{VepBuilder, VepBuildInput, VepPacket as AttestVepPacket};
use serde_json::json;
use snow::Builder;
use std::sync::Arc;
use vex_core::vep::VepPacket;

#[tokio::main]
async fn main() {
    println!("Starting Cross-Project VEP Parity Test...");

    // 1. Initialize Mock Transport for Attest-RS VepBuilder
    let pattern: snow::params::NoiseParams = "Noise_XX_25519_ChaChaPoly_BLAKE2b".parse().unwrap();
    let i_static = [1u8; 32];
    let r_static = [2u8; 32];
    let mut i = Builder::new(pattern.clone())
        .local_private_key(&i_static)
        .build_initiator()
        .unwrap();
    let mut r = Builder::new(pattern.clone())
        .local_private_key(&r_static)
        .build_responder()
        .unwrap();
    let mut b = [0u8; 1024];
    let mut p = [0u8; 1024];
    let len = i.write_message(&[], &mut b).unwrap();
    r.read_message(&b[..len], &mut p).unwrap();
    let len = r.write_message(&[], &mut b).unwrap();
    i.read_message(&b[..len], &mut p).unwrap();
    let len = i.write_message(&[], &mut b).unwrap();
    r.read_message(&b[..len], &mut p).unwrap();

    let _t_i = i.into_stateless_transport_mode().unwrap();

    // 2. Define Shared Data (Deterministic for Parity)
    let kp = Arc::new(MockKeyProvider);
    let mut intent = Intent::new("test-agent".into(), "test-goal".into());
    intent.created_at = chrono::DateTime::parse_from_rfc3339("2026-03-08T08:44:00Z")
        .unwrap()
        .with_timezone(&chrono::Utc);
    let auth = AuthoritySegment {
        nonce: 123456789,
        trace_root: [0xaa; 32],
    };
    let witness = AttestWitnessSegment {
        chora_node_id: "chora-alpha".into(),
        receipt_hash: "1234567890abcdef".into(),
        timestamp: 1678888888,
    };
    let identity = json!({
        "agent": "provnai-terminal",
        "tpm": "vTPM-mock"
    });
    let payload = b"Hello, ProvnAI Ecosystem!";

    // 3. Construct VEP using attest-rs
    let vep_bytes = VepBuilder::build_with_hardware_quote(VepBuildInput {
        nonce: 999,
        intent: &intent,
        auth: &auth,
        identity: &identity,
        witness: &witness,
        kp: kp.clone(),
        transport: None,
        payload,
    })
    .await
    .expect("Failed to build VEP from attest-rs");

    println!(
        "VEP Packet Constructed (attest-rs): {} bytes",
        vep_bytes.len()
    );

    // 4. Parse VEP using vex-core
    let packet = VepPacket::new(&vep_bytes).expect("Failed to parse VEP with vex-core");

    let capsule = packet
        .to_capsule()
        .expect("Failed to reconstruct Capsule in vex-core");

    // 5. Compare Roots
    let attest_packet = AttestVepPacket::from_bytes(&vep_bytes).expect("Failed attest parse");
    let vex_core_capsule_root = capsule
        .to_composite_hash()
        .expect("Failed matching vex-core composite");

    println!(
        "Attest-RS Root: {}",
        hex::encode(attest_packet.header.capsule_root)
    );
    println!("VEX-Core Root:  {}", vex_core_capsule_root.to_hex());

    assert_eq!(
        hex::encode(attest_packet.header.capsule_root),
        vex_core_capsule_root.to_hex(),
        "Capsule Roots do not match between attest-rs and vex-core!"
    );

    println!("✅ SUCCESS: Cross-project VEP Parity confirmed!");
}
