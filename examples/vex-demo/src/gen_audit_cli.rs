//! Generate valid test audit files for CLI testing
//! Run with: cargo run --example gen_audit_cli

use chrono::Utc;
use std::fs::File;
use std::io::Write;
use uuid::Uuid;
use vex_core::{Hash, MerkleTree};
use vex_persist::audit_store::{AuditEvent, AuditEventType, AuditExport};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let agent_id = Uuid::new_v4();

    // Event 1: Root Creation
    let e1 = AuditEvent::new(
        AuditEventType::AgentCreated,
        Some(agent_id),
        serde_json::json!({"role": "Root Agent", "model": "deepseek-chat"}),
        0,
    );

    // Event 2: Execution (chained)
    let e2 = AuditEvent::chained(
        AuditEventType::AgentExecuted,
        Some(agent_id),
        serde_json::json!({"prompt": "What is the meaning of life?", "confidence": 0.95}),
        e1.hash.clone(),
        1,
    );

    // Event 3: Another execution (chained)
    let e3 = AuditEvent::chained(
        AuditEventType::AgentExecuted,
        Some(agent_id),
        serde_json::json!({"prompt": "Follow-up question", "confidence": 0.87}),
        e2.hash.clone(),
        2,
    );

    let events = vec![e1.clone(), e2.clone(), e3.clone()];

    // Build Merkle Tree
    let leaves: Vec<(String, Hash)> = events
        .iter()
        .map(|e| (e.id.to_string(), e.hash.clone()))
        .collect();
    let tree = MerkleTree::from_leaves(leaves);

    let export = AuditExport {
        events: events.clone(),
        merkle_root: tree.root_hash().map(|h| h.to_string()),
        exported_at: Utc::now(),
        verified: true,
    };

    // 1. Save Valid Audit
    let valid_json = serde_json::to_string_pretty(&export)?;
    let mut file = File::create("audit_valid.json")?;
    file.write_all(valid_json.as_bytes())?;
    println!("✓ Generated audit_valid.json (3 events)");

    // 2. Save Tampered Audit (Change data but keep original hash/root)
    let mut tampered_export = export.clone();
    tampered_export.events[1].data = serde_json::json!({"prompt": "TAMPERED DATA"});
    // Note: Hash is NOT updated, so verification should FAIL

    let tampered_json = serde_json::to_string_pretty(&tampered_export)?;
    let mut file = File::create("audit_tampered.json")?;
    file.write_all(tampered_json.as_bytes())?;
    println!("✓ Generated audit_tampered.json (data tampered)");

    // 3. Save Tampered Root
    let mut tampered_root_export = export.clone();
    tampered_root_export.merkle_root = Some("fake_root_abc123".to_string());

    let tampered_root_json = serde_json::to_string_pretty(&tampered_root_export)?;
    let mut file = File::create("audit_tampered_root.json")?;
    file.write_all(tampered_root_json.as_bytes())?;
    println!("✓ Generated audit_tampered_root.json (root tampered)");

    println!("\nTest with:");
    println!("  vex verify -a audit_valid.json          # Should pass");
    println!("  vex verify -a audit_tampered.json       # Should fail (hash mismatch)");
    println!("  vex verify -a audit_tampered_root.json  # Should fail (root mismatch)");

    Ok(())
}
