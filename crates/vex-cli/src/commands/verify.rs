//! Verify command - Check audit chain integrity
//!
//! Usage:
//! ```bash
//! vex verify --audit session.json
//! vex verify --db vex.db
//! ```

use anyhow::{Context, Result};
use clap::Args;
use colored::Colorize;
use std::path::{Path, PathBuf};

/// Arguments for the verify command
#[derive(Args)]
pub struct VerifyArgs {
    /// Path to audit JSON file to verify
    #[arg(long, short = 'a', value_name = "FILE")]
    audit: Option<PathBuf>,

    /// Path to VEX database file
    #[arg(long, short = 'd', value_name = "FILE")]
    db: Option<PathBuf>,

    /// Show detailed verification output
    #[arg(long)]
    detailed: bool,

    /// Perform a live witness handshake audit (e.g., against CHORA)
    #[arg(long, short = 'l')]
    live: bool,

    /// CHORA Gate URL for live audit
    #[arg(long, env = "CHORA_GATE_URL")]
    gate_url: Option<String>,

    /// CHORA API Key for live audit
    #[arg(long, env = "CHORA_API_KEY")]
    api_key: Option<String>,

    /// Path to a binary .vep or .json capsule for local verification
    #[arg(long, short = 'c', value_name = "FILE")]
    capsule: Option<PathBuf>,

    /// Authority public key (hex) for local signature verification
    #[arg(long, short = 'p', value_name = "HEX")]
    public_key: Option<String>,
}

/// Run the verify command
pub async fn run(args: VerifyArgs) -> Result<()> {
    if args.audit.is_none() && args.db.is_none() && !args.live && args.capsule.is_none() {
        println!("{}", "VEX Verification".bold().cyan());
        println!("{}", "═".repeat(40).cyan());
        println!();
        println!("Usage:");
        println!(
            "  {} Verify an exported audit file",
            "vex verify --audit session.json".green()
        );
        println!(
            "  {} Verify a VEX database",
            "vex verify --db vex.db".green()
        );
        println!(
            "  {} Verify a binary or JSON capsule",
            "vex verify --capsule capsule.vep".green()
        );
        println!(
            "  {} Perform live witness handshake audit",
            "vex verify --live".green()
        );
        println!();
        println!("The verify command checks the integrity of VEX audit chains");
        println!("using Merkle tree verification or local forensic analysis.");
        return Ok(());
    }

    // Handle audit file verification
    if let Some(audit_path) = &args.audit {
        verify_audit_file(audit_path, args.detailed).await?;
    }

    // Handle database verification
    if let Some(db_path) = &args.db {
        verify_database(db_path, args.detailed).await?;
    }

    // Handle live verification
    if args.live {
        verify_live_handshake(args.gate_url.as_deref(), args.api_key.as_deref()).await?;
    }

    // Handle local capsule verification
    if let Some(capsule_path) = &args.capsule {
        verify_capsule_file(capsule_path, args.public_key.as_deref(), args.detailed).await?;
    }

    Ok(())
}

/// Perform a live witness handshake audit against a production gate
async fn verify_live_handshake(gate_url: Option<&str>, api_key: Option<&str>) -> Result<()> {
    use sha2::Digest;
    use uuid::Uuid;
    use vex_chora::client::{AuthorityClient, HttpChoraClient};
    use vex_core::segment::IntentData;

    println!("{}", "🌐 VEX Live Witness Audit".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    let gate_url_str = gate_url.unwrap_or("https://gate.witness.network"); // Generic Placeholder

    let client = HttpChoraClient::new(gate_url_str.to_string(), api_key.unwrap_or("").to_string());

    println!("  {} {}", "Gate:".dimmed(), gate_url_str);
    print!("  Generating random intent... ");

    let nonce = Uuid::new_v4().to_string();
    let intent = IntentData::Transparent {
        request_sha256: hex::encode(sha2::Sha256::digest(b"Live CLI Handshake Audit")),
        confidence: 0.99,
        capabilities: vec!["live-cli-audit".to_string()],
        magpie_source: None,
        metadata: serde_json::json!({ "nonce": nonce, "context": "cli-audit" }),
    };

    println!("{}", "DONE".green());
    println!("  {} {}", "Audit Nonce:".dimmed(), nonce.yellow());

    print!("  Requesting attestation... ");
    let intent_jcs = serde_jcs::to_vec(&intent).context("Failed to canonicalize intent")?;
    let resp = client
        .request_attestation(&intent_jcs)
        .await
        .map_err(|e| anyhow::anyhow!("Handshake failed: {}", e))?;

    println!("{}", "RECEIVED".green());
    println!(
        "  {} {}",
        "Capsule ID:".dimmed(),
        resp.authority.capsule_id.bold()
    );
    println!(
        "  {} {}",
        "Outcome:".dimmed(),
        resp.authority.outcome.green().bold()
    );

    // Fetch full capsule to get all hashes for root reconstruction
    print!("  Fetching full evidence... ");
    let capsule_url = format!(
        "{}/capsules/{}/json",
        gate_url_str.trim_end_matches('/'),
        resp.authority.capsule_id
    );
    let full_capsule_json: serde_json::Value = reqwest::get(&capsule_url)
        .await
        .context("Failed to connect to capsule endpoint")?
        .json()
        .await
        .context("Failed to parse full capsule JSON")?;
    println!("{}", "DONE".green());

    let reported_root = full_capsule_json["capsule_root"]
        .as_str()
        .context("Missing capsule_root in response")?;

    print!("  Auditing root commitment... ");
    // Reconstruct root locally using reported hashes (v0.3 Merkle Tree model)
    use vex_core::merkle::{Hash, MerkleTree};

    let leaves = vec![
        (
            "intent".to_string(),
            Hash::from_bytes(
                hex::decode(full_capsule_json["intent_hash"].as_str().unwrap_or(""))
                    .map_err(|_| anyhow::anyhow!("Invalid intent_hash"))?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid intent_hash length"))?,
            ),
        ),
        (
            "authority".to_string(),
            Hash::from_bytes(
                hex::decode(full_capsule_json["authority_hash"].as_str().unwrap_or(""))
                    .map_err(|_| anyhow::anyhow!("Invalid authority_hash"))?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid authority_hash length"))?,
            ),
        ),
        (
            "identity".to_string(),
            Hash::from_bytes(
                hex::decode(full_capsule_json["identity_hash"].as_str().unwrap_or(""))
                    .map_err(|_| anyhow::anyhow!("Invalid identity_hash"))?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid identity_hash length"))?,
            ),
        ),
        (
            "witness".to_string(),
            Hash::from_bytes(
                hex::decode(full_capsule_json["witness_hash"].as_str().unwrap_or(""))
                    .map_err(|_| anyhow::anyhow!("Invalid witness_hash"))?
                    .try_into()
                    .map_err(|_| anyhow::anyhow!("Invalid witness_hash length"))?,
            ),
        ),
    ];

    let tree = MerkleTree::from_leaves(leaves);
    let local_root = tree
        .root_hash()
        .map(|h| h.to_hex())
        .ok_or_else(|| anyhow::anyhow!("Failed to calculate Merkle root"))?;

    if local_root != reported_root {
        return Err(anyhow::anyhow!(
            "Root audit failure! Local: {}, Reported: {}",
            local_root,
            reported_root
        ));
    }
    println!("{}", "OK".green());

    print!("  Verifying production signature... ");
    let sig_hex = full_capsule_json["crypto"]["signature_b64"]
        .as_str()
        .or_else(|| full_capsule_json["signature"].as_str())
        .context("Missing signature in evidence")?;

    let sig_bytes = if sig_hex.len() == 128 {
        hex::decode(sig_hex).context("Failed to decode hex signature")?
    } else {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, sig_hex)
            .context("Failed to decode base64 signature")?
    };

    let root_bytes = hex::decode(&local_root).context("Failed to decode local root to bytes")?;
    let verified = client
        .verify_witness_signature(&root_bytes, &sig_bytes)
        .await
        .map_err(|e| anyhow::anyhow!("Signature verification error: {}", e))?;

    if !verified {
        return Err(anyhow::anyhow!(
            "Cryptographic verification failed! Signature is invalid for the calculated root."
        ));
    }
    println!("{}", "VERIFIED".green());

    println!();
    println!("{} Live handshake audit successful.", "✓".green().bold());
    println!(
        "  All cryptographic guarantees verified against {} gate.",
        "CHORA".bold()
    );

    Ok(())
}

/// Verify an exported audit JSON file
async fn verify_audit_file(path: &std::path::PathBuf, detailed: bool) -> Result<()> {
    use vex_core::audit::AuditEvent;
    use vex_core::{Hash, MerkleTree};
    use vex_persist::audit_store::AuditExport;

    println!("{}", "🔐 VEX Audit Verification".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    // Read and parse the audit file
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read audit file: {}", path.display()))?;

    let audit_data: AuditExport =
        serde_json::from_str(&content).with_context(|| "Failed to parse audit JSON")?;

    let events = &audit_data.events;
    let event_count = events.len();

    // Summary info
    println!("  {} {}", "File:".dimmed(), path.display());
    println!("  {} {}", "Events:".dimmed(), event_count);
    println!(
        "  {} {}",
        "Merkle Root (File):".dimmed(),
        audit_data.merkle_root.as_deref().unwrap_or("None")
    );
    println!();

    if events.is_empty() {
        println!("  {}", "Warning: No events to verify".yellow());
        return Ok(());
    }

    // 1. Verify individual event hashes and the chain
    let mut last_hash: Option<Hash> = None;
    for (i, event) in events.iter().enumerate() {
        // Re-calculate the "individual" hash (Centralized in vex-core)
        use vex_core::audit::HashParams;
        let base_hash = AuditEvent::compute_hash(HashParams {
            event_type: &event.event_type,
            timestamp: event.timestamp.timestamp(),
            sequence_number: event.sequence_number,
            data: &event.data,
            actor: &event.actor,
            rationale: &event.rationale,
            policy_version: &event.policy_version,
            data_provenance_hash: &event.data_provenance_hash,
            human_review_required: event.human_review_required,
            approval_count: event.approval_signatures.len(),
            evidence_capsule: &event.evidence_capsule,
            schema_version: &event.schema_version,
        });

        // Calculate expected final hash (including chain link if applicable)
        let expected_hash = if let Some(prev) = &event.previous_hash {
            if i == 0 {
                return Err(anyhow::anyhow!(
                    "Audit failure: First event has a previous_hash link"
                ));
            }
            if let Some(actual_prev) = &last_hash {
                if prev != actual_prev {
                    return Err(anyhow::anyhow!(
                        "Chain integrity failure at event {}: expected previous_hash {}, got {}",
                        event.id,
                        actual_prev.to_hex(),
                        prev.to_hex()
                    ));
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Chain integrity failure: previous_hash present but last_hash missing"
                ));
            }

            // Chained hash logic (Centralized in vex-core)
            AuditEvent::compute_chained_hash(&base_hash, prev, event.sequence_number)
        } else {
            if i > 0 {
                return Err(anyhow::anyhow!(
                    "Chain integrity failure: Event {} is missing previous_hash link",
                    event.id
                ));
            }
            base_hash
        };

        // Compare with the hash in the file
        if expected_hash != event.hash {
            return Err(anyhow::anyhow!(
                "Event hash mismatch at event {}: expected {}, got {}",
                event.id,
                expected_hash.to_hex(),
                event.hash.to_hex()
            ));
        }

        last_hash = Some(event.hash.clone());
    }

    // 2. Build Merkle tree from verified hashes
    let leaves: Vec<(String, Hash)> = events
        .iter()
        .map(|e| (e.id.to_string(), e.hash.clone()))
        .collect();
    let tree = MerkleTree::from_leaves(leaves);
    let calculated_root = tree.root_hash().map(|h| h.to_string());

    // 3. Compare roots
    match (&audit_data.merkle_root, &calculated_root) {
        (Some(file_root), Some(calc_root)) => {
            if file_root != calc_root {
                return Err(anyhow::anyhow!(
                    "Merkle root mismatch! File: {}, Calculated: {}",
                    file_root,
                    calc_root
                ));
            }
        }
        (None, None) => {}
        _ => {
            return Err(anyhow::anyhow!(
                "Merkle root presence mismatch between file and calculation"
            ));
        }
    }

    println!(
        "{} {} verified successfully.",
        "✓".green().bold(),
        "Merkle tree & Audit chain".bold()
    );

    if detailed {
        println!();
        println!("{}", "Event Detail Log:".bold());
        for (i, event) in events.iter().take(10).enumerate() {
            println!(
                "  {}. {} [{}] @ {}",
                i + 1,
                format!("{:?}", event.event_type).yellow(),
                event.hash.to_hex()[..8].dimmed(),
                event.timestamp.to_rfc3339().dimmed()
            );
        }
        if events.len() > 10 {
            println!("  ... and {} more events", events.len() - 10);
        }
    }

    Ok(())
}

/// Verify a VEX database file
async fn verify_database(path: &Path, detailed: bool) -> Result<()> {
    println!("{}", "🔐 VEX Database Verification".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    if !path.exists() {
        println!(
            "{} Database file not found: {}",
            "✗".red().bold(),
            path.display()
        );
        std::process::exit(1);
    }

    println!("  {} {}", "Database:".dimmed(), path.display());

    println!("  {} {}", "Database:".dimmed(), path.display());

    // Connect to database
    let db_url = format!("sqlite://{}", path.display());
    let backend = vex_persist::sqlite::SqliteBackend::new(&db_url)
        .await
        .map_err(|e| anyhow::anyhow!("Failed to initialize backend for proof export: {}", e))?;
    backend
        .migrate()
        .await
        .map_err(|e| anyhow::anyhow!("Failed to migrate backend: {}", e))?;

    let store = vex_persist::audit_store::AuditStore::new(std::sync::Arc::new(backend));

    // In a real multi-tenant system, we'd need a list of tenants.
    // For the CLI tool, we'll try to find common tenants or verify the default ones.
    // For now, let's look for all unique tenant_ids in the database.
    let pool = vex_persist::sqlite::SqliteBackend::new(&db_url)
        .await?
        .pool()
        .clone();
    let tenants: Vec<String> =
        sqlx::query_as::<_, (String,)>("SELECT DISTINCT tenant_id FROM audit_events")
            .fetch_all(&pool)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(t,)| t)
            .collect();

    if tenants.is_empty() {
        println!(
            "  {}",
            "Warning: No audit events found in database".yellow()
        );
        return Ok(());
    }

    println!("  {} {}", "Tenants found:".dimmed(), tenants.len());
    println!();

    for tenant in tenants {
        print!("  Verifying tenant {}... ", tenant.bold());
        match store.verify_chain(&tenant).await {
            Ok(true) => println!("{}", "OK".green()),
            Ok(false) => println!("{}", "FAILED (Integrity break)".red()),
            Err(e) => println!("{} ({})", "ERROR".red(), e),
        }

        if detailed {
            let tree = store.build_merkle_tree(&tenant).await?;
            println!(
                "    Merkle Root: {}",
                tree.root_hash()
                    .map(|h| h.to_hex())
                    .unwrap_or_else(|| "None".to_string())
            );
            let count = store.get_chain(&tenant).await?.len();
            println!("    Event Count: {}", count);
        }
    }

    println!();
    println!("{} Database verification complete.", "✓".green().bold());

    Ok(())
}

/// Verify a binary .vep or JSON .json capsule locally
async fn verify_capsule_file(
    path: &Path,
    public_key_hex: Option<&str>,
    _detailed: bool,
) -> Result<()> {
    use ed25519_dalek::{Signature, Verifier, VerifyingKey};
    use vex_core::segment::Capsule;
    use vex_core::vep::VepPacket;

    println!("{}", "🔍 VEX Local Capsule Audit".bold().cyan());
    println!("{}", "═".repeat(40).cyan());
    println!();

    let bytes =
        std::fs::read(path).with_context(|| format!("Failed to read file: {}", path.display()))?;

    // 1. Parse and extract metadata
    let capsule = if bytes.starts_with(&vex_core::vep::VEP_MAGIC) || bytes.starts_with(b"EPH") {
        print!("  Detecting Format: ");
        println!("{}", "BINARY (V1.0)".green().bold());
        let packet =
            VepPacket::new(&bytes).map_err(|e| anyhow::anyhow!("Binary parse error: {}", e))?;
        packet
            .to_capsule()
            .map_err(|e| anyhow::anyhow!("Capsule reconstruction failed: {}", e))?
    } else {
        print!("  Detecting Format: ");
        println!("{}", "JSON (V0.2)".green().bold());
        serde_json::from_slice::<Capsule>(&bytes).context("Failed to parse JSON capsule")?
    };

    println!("  {} {}", "Capsule ID:".dimmed(), capsule.capsule_id);
    println!(
        "  {} {}",
        "Outcome:".dimmed(),
        capsule.authority.outcome.green().bold()
    );

    // 2. Merkle Audit (Recompute Root)
    print!("  Auditing Root Commitment... ");
    let root_hash = capsule
        .to_composite_hash()
        .map_err(|e| anyhow::anyhow!("Merkle reconstruction error: {}", e))?;
    let mut _root_hex = root_hash.to_hex();
    if _root_hex != capsule.capsule_root {
        println!("{}", "FAILED".red().bold());
        println!("  {} {}", "Calculated Root:".dimmed(), _root_hex);
        println!("  {} {}", "Expected Root:  ".dimmed(), capsule.capsule_root);
        return Err(anyhow::anyhow!(
            "Merkle root mismatch! The data does not match the commitment (Strict v0.3 enforced)."
        ));
    } else {
        println!("{}", "OK".green().bold());
    }
    // 3. Witness Signature Verification
    if let Some(pk_hex) = public_key_hex {
        print!("  Verifying Witness Signature... ");
        let pk_bytes = hex::decode(pk_hex).context("Invalid public key hex")?;
        let pk: [u8; 32] = pk_bytes
            .try_into()
            .map_err(|_| anyhow::anyhow!("Public key must be 32 bytes"))?;
        let public_key = VerifyingKey::from_bytes(&pk).context("Invalid Ed25519 public key")?;

        // Extract signature from CryptoData
        let sig_bytes = if capsule.crypto.signature_b64.len() == 128 {
            hex::decode(&capsule.crypto.signature_b64).context("Failed to decode hex signature")?
        } else {
            base64::Engine::decode(
                &base64::engine::general_purpose::STANDARD,
                &capsule.crypto.signature_b64,
            )
            .context("Failed to decode base64 signature")?
        };

        if sig_bytes.len() != 64 {
            return Err(anyhow::anyhow!(
                "Invalid signature length (expected 64 bytes, got {})",
                sig_bytes.len()
            ));
        }

        let signature = Signature::from_slice(&sig_bytes).context("Failed to parse signature")?;
        let root_bytes = root_hash.0;

        match public_key.verify(&root_bytes, &signature) {
            Ok(_) => println!("{}", "VERIFIED".green().bold()),
            Err(_) => {
                return Err(anyhow::anyhow!(
                    "Cryptographic audit failure! Signature is invalid."
                ))
            }
        }
    } else {
        println!(
            "  {} {}",
            "Witness Status:".dimmed(),
            "PENDING (Public Key not provided)".yellow()
        );
    }

    // 4. Identity Audit
    if let Some(pcrs) = &capsule.identity.pcrs {
        println!("  {} Silicon-Bound (PCRs Present)", "Identity:".dimmed());
        if !pcrs.is_empty() {
            println!("    {}", "Captured PCRs:".dimmed());
            let mut indices: Vec<_> = pcrs.keys().collect();
            indices.sort();
            for idx in indices {
                println!("      PCR {}: {}", idx, pcrs[idx].dimmed());
            }
        }
    } else {
        println!("  {} {}", "Identity:".dimmed(), capsule.identity.aid.blue());
    }

    println!();
    println!(
        "{} Local forensic verification complete.",
        "✓".green().bold()
    );

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use uuid::Uuid;
    use vex_core::audit::{AuditEvent, AuditEventType};
    use vex_core::{Hash, MerkleTree};
    use vex_persist::audit_store::AuditExport;

    fn create_test_audit() -> AuditExport {
        let agent_id = Uuid::new_v4();
        let e1 = AuditEvent::new(
            AuditEventType::AgentCreated,
            Some(agent_id),
            serde_json::json!({"role": "Root"}),
            0,
        );
        let e2 = AuditEvent::chained(
            AuditEventType::AgentExecuted,
            Some(agent_id),
            serde_json::json!({"prompt": "Hello"}),
            e1.hash.clone(),
            1,
        );
        let events = vec![e1.clone(), e2.clone()];
        let leaves: Vec<(String, Hash)> = events
            .iter()
            .map(|e| (e.id.to_string(), e.hash.clone()))
            .collect();
        let tree = MerkleTree::from_leaves(leaves);

        AuditExport {
            events,
            merkle_root: tree.root_hash().map(|h| h.to_string()),
            exported_at: Utc::now(),
            verified: true,
        }
    }

    #[tokio::test]
    async fn test_verify_valid_audit() {
        let export = create_test_audit();
        let path = std::env::temp_dir().join("audit_valid.json");
        let json = serde_json::to_string(&export).unwrap();
        std::fs::write(&path, json).unwrap();

        let result = verify_audit_file(&path, false).await;
        assert!(
            result.is_ok(),
            "Valid audit should verify! Error: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_verify_tampered_data() {
        let mut export = create_test_audit();
        // Tamper with data without updating hash
        export.events[0].data = serde_json::json!({"role": "TAMPERED"});

        let path = std::env::temp_dir().join("audit_tampered_data.json");
        let json = serde_json::to_string(&export).unwrap();
        std::fs::write(&path, json).unwrap();

        let result = verify_audit_file(&path, false).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Event hash mismatch"));
    }

    #[tokio::test]
    async fn test_verify_tampered_root() {
        let mut export = create_test_audit();
        // Tamper with root
        export.merkle_root = Some("fake_root".to_string());

        let path = std::env::temp_dir().join("audit_tampered_root.json");
        let json = serde_json::to_string(&export).unwrap();
        std::fs::write(&path, json).unwrap();

        let result = verify_audit_file(&path, false).await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Merkle root mismatch"));
    }

    #[tokio::test]
    async fn test_verify_capsule_offline() {
        use ed25519_dalek::{Signer, SigningKey};
        use rand::RngCore;
        use std::collections::HashMap;
        use vex_core::segment::{
            AuthorityData, Capsule, CryptoData, IdentityData, IntentData, WitnessData,
        };

        let mut rng = rand::thread_rng();
        let mut seed = [0u8; 32];
        rng.fill_bytes(&mut seed);
        let signing_key = SigningKey::from_bytes(&seed);
        let public_key = signing_key.verifying_key();

        let mut capsule = Capsule {
            capsule_id: "test-capsule".to_string(),
            intent: IntentData::Transparent {
                request_sha256: "hash".to_string(),
                confidence: 1.0,
                capabilities: vec![],
                magpie_source: None,
                metadata: serde_json::Value::Null,
            },
            authority: AuthorityData {
                capsule_id: "test-capsule".to_string(),
                outcome: "ALLOW".to_string(),
                reason_code: "OK".to_string(),
                trace_root: "trace".to_string(),
                nonce: 123,
                gate_sensors: serde_json::Value::Null,
                metadata: serde_json::Value::Null,
                escalation_id: None,
                continuation_token: None,
                binding_status: None,
            },
            identity: IdentityData {
                aid: "test-aid".to_string(),
                identity_type: "unbound".to_string(),
                pcrs: Some(HashMap::new()),
                metadata: serde_json::Value::Null,
            },
            witness: WitnessData {
                chora_node_id: "test-node".to_string(),
                receipt_hash: "test-receipt".to_string(),
                timestamp: 123456789,
                metadata: serde_json::Value::Null,
            },
            intent_hash: "hash".to_string(), // Initialized for JCS
            authority_hash: "hash".to_string(),
            identity_hash: "hash".to_string(),
            witness_hash: "hash".to_string(),
            capsule_root: String::new(),
            crypto: CryptoData {
                algo: "ed25519".to_string(),
                public_key_endpoint: "".to_string(),
                signature_scope: "capsule_root".to_string(),
                signature_b64: String::new(),
            },
            request_commitment: None,
        };

        // Recompute hashes and root
        let root_hash = capsule.to_composite_hash().unwrap();
        capsule.capsule_root = root_hash.to_hex();

        // Sign the root
        let signature = signing_key.sign(&root_hash.0);
        capsule.crypto.signature_b64 = hex::encode(signature.to_bytes());

        let path = std::env::temp_dir().join("test_capsule.json");
        let json = serde_json::to_string(&capsule).unwrap();
        std::fs::write(&path, json).unwrap();

        let pk_hex = hex::encode(public_key.to_bytes());
        let result = verify_capsule_file(&path, Some(&pk_hex), false).await;

        assert!(
            result.is_ok(),
            "Offline verification should succeed! Error: {:?}",
            result.err()
        );
    }
}
