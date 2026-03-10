use attest_rs::{ActorType, AttestAgent, AuditEventType, AuditStore, LocalStore};
use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use std::sync::Arc;

#[tokio::test]
async fn test_tamper_proof_chain() -> anyhow::Result<()> {
    // 1. Setup InMemory DB or File DB
    // We use a file DB to make it easier to corrupt via "side-channel" connection if needed,
    // but SQLx in-memory is fine if we use the same pool to execute raw SQL.
    let _pool = SqlitePoolOptions::new().connect("sqlite::memory:").await?;

    // 1. Setup InMemory DB or File DB
    // Use unique path to avoid zombie locks on Windows
    let current_dir = std::env::current_dir()?;
    let db_name = format!("test_tamper_{}.db", uuid::Uuid::new_v4());
    let db_path = current_dir.join("target").join(&db_name);

    // Clean up
    if db_path.exists() {
        // We retry remove in case of linger locks from previous runs
        let _ = std::fs::remove_file(&db_path);
        let _ = std::fs::remove_file(format!("{}-shm", db_path.display()));
        let _ = std::fs::remove_file(format!("{}-wal", db_path.display()));
    }

    // Ensure parent dir exists
    if let Some(p) = db_path.parent() {
        std::fs::create_dir_all(p)?;
    }

    let db_str = db_path.to_str().unwrap();
    println!("🧪 Test Database: {}", db_str);

    let local = Arc::new(LocalStore::new(db_str).await?);
    let audit = AuditStore::new(local.clone());
    let agent = AttestAgent::new();

    // 2. Generate Legitimate Logs (Genesis + 2 Events)
    audit
        .log(
            AuditEventType::AgentCreated,
            None,
            json!({"status": "born"}),
            ActorType::System,
            &agent,
        )
        .await?;

    audit
        .log(
            AuditEventType::AgentExecuted,
            None,
            json!({"cmd": "echo hello"}),
            ActorType::System,
            &agent,
        )
        .await?;

    // 3. Verify Integrity (Should be valid)
    println!("🕵️ Verifying Initial Integrity (Expecting Success)...");
    let is_valid = audit.verify_integrity().await?;
    println!("🕵️ Initial Verification Result: {}", is_valid);
    assert!(is_valid, "Initial chain must be valid");

    // 4. ATTACK: Manually Corrupt Event #2 (Sequence 1)
    // Use the existing connection to avoid Windows file locking issues
    println!("⚔️ Executing SQL Injection Attack...");
    local
        .execute_raw_sql(
            r#"UPDATE audit_events SET data = '{"cmd": "echo pwnd"}' WHERE sequence_number = 1"#,
        )
        .await?;
    println!("⚔️ Attack Complete.");

    // 5. Verify Integrity (Should Fail)
    println!("🕵️ Verifying Integrity (Expecting Failure)...");
    let is_valid_after_attack = audit.verify_integrity().await?;
    println!("🕵️ Verification Result: {}", is_valid_after_attack);
    assert!(
        !is_valid_after_attack,
        "Chain verification MUST fail after data substitution"
    );

    println!("✅ Tamper-Proof Test Passed: Modification detected.");

    Ok(())
}
