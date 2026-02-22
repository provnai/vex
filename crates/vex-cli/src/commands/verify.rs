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
}

/// Run the verify command
pub async fn run(args: VerifyArgs) -> Result<()> {
    if args.audit.is_none() && args.db.is_none() {
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
        println!();
        println!("The verify command checks the integrity of VEX audit chains");
        println!("using Merkle tree verification.");
        return Ok(());
    }

    // Handle audit file verification
    if let Some(audit_path) = args.audit {
        verify_audit_file(&audit_path, args.detailed).await?;
    }

    // Handle database verification
    if let Some(db_path) = args.db {
        verify_database(&db_path, args.detailed).await?;
    }

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
            timestamp: event.timestamp,
            sequence_number: event.sequence_number,
            data: &event.data,
            actor: &event.actor,
            rationale: &event.rationale,
            policy_version: &event.policy_version,
            data_provenance_hash: &event.data_provenance_hash,
            human_review_required: event.human_review_required,
            approval_count: event.approval_signatures.len(),
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
        .with_context(|| format!("Failed to connect to database: {}", path.display()))?;

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
    use vex_core::audit::{ActorType, AuditEvent, AuditEventType};
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
