use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use serde_json::json;
use std::path::PathBuf;
use vex_core::segment::IntentData;

/// Generate mock ZK proofs for debugging the VEP Explorer and SDK pipeline
#[derive(Parser)]
pub struct ProveArgs {
    /// The intent text to "hide" behind a Shadow proof
    #[arg(
        short,
        long,
        default_value = "Mock Intent Payload for VEX Explorer Debugging"
    )]
    intent: String,

    /// Output path for the generated mock Shadow intent (JSON)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Generate a full mock Capsule instead of just an Intent segment
    #[arg(long)]
    full_capsule: bool,
}

pub async fn run(args: ProveArgs) -> Result<()> {
    println!(
        "{}",
        "🏛️ VEX ZK Debugger (Mock Proof Generation)".bold().purple()
    );
    println!("{}", "═".repeat(40).purple());
    println!();

    println!("  {} {}", "Target Intent:".dimmed(), args.intent);

    // 1. Generate Commitment Hash (SHA-256 of the intent)
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(args.intent.as_bytes());
    let commitment = hex::encode(hasher.finalize());
    println!("  {} {}", "Commitment:".dimmed(), commitment.yellow());

    // 2. Generate Mock STARK Proof
    // Base64 of "vex-stark-mock-proof-for-explorer-debugging"
    let mock_stark = "dmV4LXN0YXJrLW1vY2stcHJvb2YtZm9yLWV4cGxvcmVyLWRlYnVnZ2luZy1zdGFiaWxpdHk=";
    println!(
        "  {} {}",
        "STARK Proof:".dimmed(),
        "[MOCK-PLONKY3-STARK]".cyan()
    );

    // 3. Construct Shadow Intent
    let shadow_intent = IntentData::Shadow {
        commitment_hash: commitment,
        stark_proof_b64: mock_stark.to_string(),
        public_inputs: json!({
            "policy_id": "standard-v1",
            "outcome_commitment": "ALLOW",
            "intent_hash_ref": args.intent
        }),
        metadata: json!({
            "debugger_mode": true,
            "timestamp": chrono::Utc::now().to_rfc3339()
        }),
        circuit_id: None,
    };

    if args.full_capsule {
        use std::collections::HashMap;
        use vex_core::segment::{AuthorityData, Capsule, CryptoData, IdentityData, WitnessData};

        let mut capsule = Capsule {
            capsule_id: format!("mock-zk-{}", uuid::Uuid::new_v4()),
            intent: shadow_intent,
            authority: AuthorityData {
                capsule_id: "mock-zk-capsule".to_string(),
                outcome: "ALLOW".to_string(),
                reason_code: "OK".to_string(),
                trace_root: "0xdeadbeef".to_string(),
                nonce: 42,
                escalation_id: None,
                binding_status: None,
                continuation_token: None,
                gate_sensors: json!({ "tpm_active": true }),
                metadata: serde_json::Value::Null,
            },
            identity: IdentityData {
                aid: "0x1234567890abcdef".to_string(),
                identity_type: "unbound".to_string(),
                pcrs: Some(HashMap::new()),
                metadata: serde_json::Value::Null,
            },
            witness: WitnessData {
                chora_node_id: "vex-debug-node".to_string(),
                receipt_hash: "0xabcdef".to_string(),
                timestamp: chrono::Utc::now().timestamp() as u64,
                metadata: serde_json::Value::Null,
            },
            intent_hash: String::new(),
            authority_hash: String::new(),
            identity_hash: String::new(),
            witness_hash: String::new(),
            capsule_root: String::new(),
            crypto: CryptoData {
                algo: "ed25519".to_string(),
                public_key_endpoint: "".to_string(),
                signature_scope: "capsule_root".to_string(),
                signature_b64: String::new(),
            },
            request_commitment: None,
        };

        // Compute hashes
        let root_hash = capsule
            .to_composite_hash()
            .map_err(|e| anyhow::anyhow!("Hash error: {}", e))?;
        capsule.capsule_root = root_hash.to_hex();

        let json_output = serde_json::to_string_pretty(&capsule)?;

        if let Some(path) = args.output {
            std::fs::write(&path, &json_output).context("Failed to write capsule")?;
            println!(
                "  {} {}",
                "✓ Mock Capsule saved to:".green(),
                path.display()
            );
        } else {
            println!("\n{}", "Generated Mock Capsule (JSON):".bold().white());
            println!("{}", json_output);
        }
    } else {
        let json_output = serde_json::to_string_pretty(&shadow_intent)?;
        if let Some(path) = args.output {
            std::fs::write(&path, &json_output).context("Failed to write intent")?;
            println!(
                "  {} {}",
                "✓ Mock Shadow Intent saved to:".green(),
                path.display()
            );
        } else {
            println!("\n{}", "Generated Shadow Intent (JSON):".bold().white());
            println!("{}", json_output);
        }
    }

    Ok(())
}
