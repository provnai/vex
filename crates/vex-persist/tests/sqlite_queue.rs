use serde_json::json;
use sqlx::sqlite::SqlitePoolOptions;
use vex_persist::queue::SqliteQueueBackend;
use vex_queue::QueueBackend;

#[tokio::test]
async fn test_sqlite_queue_lexical_comparison_regression() -> Result<(), Box<dyn std::error::Error>>
{
    // 1. Setup DB (using a file-based DB in a temp directory if possible, but :memory: is fine for this SQL logic)
    let pool = SqlitePoolOptions::new().connect("sqlite::memory:").await?;

    // 2. Initialize schema (manually for this isolated test)
    sqlx::query(
        r#"
        CREATE TABLE jobs (
            id TEXT PRIMARY KEY,
            tenant_id TEXT NOT NULL,
            job_type TEXT NOT NULL,
            payload TEXT NOT NULL,
            status TEXT NOT NULL,
            run_at DATETIME NOT NULL,
            locked_at DATETIME,
            locked_by TEXT,
            retries INTEGER DEFAULT 0,
            last_error TEXT,
            result TEXT,
            priority INTEGER DEFAULT 0,
            created_at DATETIME DEFAULT CURRENT_TIMESTAMP
        );
        "#,
    )
    .execute(&pool)
    .await?;

    let backend = SqliteQueueBackend::new(pool);
    let tenant_id = "test-tenant";
    let payload = json!({"test": "data"});

    // 3. Enqueue a job to run NOW
    // This will insert an ISO-8601 string like "2026-02-27T22:20:00.000Z"
    let job_id = backend
        .enqueue(tenant_id, "test_job", payload, None)
        .await?;

    // 4. Dequeue immediately
    // If the datetime() fix in queue.rs is working, this should return Some(job)
    let job = backend.dequeue().await?;

    assert!(job.is_some(), "Job should have been dequeued. If this failed, the lexical comparison (T vs space) likely failed.");
    let job = job.unwrap();
    assert_eq!(job.id, job_id);

    Ok(())
}
