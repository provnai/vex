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
}

/// Run the verify command
pub async fn run(args: VerifyArgs) -> Result<()> {
    if args.audit.is_none() && args.db.is_none() && !args.live {
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
            "  {} Perform live witness handshake audit",
            "vex verify --live".green()
        );
        println!();
        println!("The verify command checks the integrity of VEX audit chains");
        println!("using Merkle tree verification or live gate auditing.");
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
    let intent = IntentData {
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
    // Reconstruct root locally using reported hashes
    let root_map = serde_json::json!({
        "authority_hash": full_capsule_json["authority_hash"].as_str().unwrap_or(""),
        "identity_hash": full_capsule_json["identity_hash"].as_str().unwrap_or(""),
        "intent_hash": full_capsule_json["intent_hash"].as_str().unwrap_or(""),
        "witness_hash": full_capsule_json["witness_hash"].as_str().unwrap_or("")
    });

    let local_root_jcs = serde_jcs::to_vec(&root_map).context("Failed to canonicalize root map")?;
    let mut hasher = sha2::Sha256::new();
    hasher.update(&local_root_jcs);
    let local_root = hex::encode(hasher.finalize());

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
}
