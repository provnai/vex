use serde::Serialize;
use sha2::{Digest, Sha256};
use serde_jcs;

#[derive(Serialize)]
struct MinimalWitness<'a> {
    chora_node_id: &'a str,
    receipt_hash: &'a str,
    timestamp: u64,
}

fn main() {
    let minimal = MinimalWitness {
        chora_node_id: "chora-gate-v1",
        receipt_hash: "",
        timestamp: 1710396000,
    };

    let jcs_bytes = serde_jcs::to_vec(&minimal).unwrap();
    let jcs_str = String::from_utf8(jcs_bytes).unwrap();
    println!("JCS: {}", jcs_str);

    let mut hasher = Sha256::new();
    hasher.update(jcs_str.as_bytes());
    let hash = hex::encode(hasher.finalize());
    println!("Hash: {}", hash);
}
