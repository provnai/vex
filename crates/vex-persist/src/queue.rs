use async_trait::async_trait;
use serde_json::Value;
use sqlx::SqlitePool;
use uuid::Uuid;
use chrono::{Utc, DateTime};
use vex_queue::{QueueBackend, backend::QueueError, job::{JobEntry, JobStatus}};

/// Durable queue backend using SQLite
pub struct SqliteQueueBackend {
    pool: SqlitePool,
}

impl SqliteQueueBackend {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl QueueBackend for SqliteQueueBackend {
    async fn enqueue(&self, job_type: &str, payload: Value, delay_secs: Option<u64>) -> Result<Uuid, QueueError> {
        let id = Uuid::new_v4();
        let run_at = if let Some(delay) = delay_secs {
            Utc::now() + chrono::Duration::seconds(delay as i64)
        } else {
            Utc::now()
        };

        sqlx::query(
            "INSERT INTO jobs (id, job_type, payload, status, run_at) VALUES (?, ?, ?, 'pending', ?)"
        )
        .bind(id.to_string())
        .bind(job_type)
        .bind(payload)
        .bind(run_at)
        .execute(&self.pool)
        .await
        .map_err(|e| QueueError::Backend(e.to_string()))?;

        Ok(id)
    }

    async fn dequeue(&self) -> Result<Option<JobEntry>, QueueError> {
        let worker_id = Uuid::new_v4().to_string(); 
        
        let row = sqlx::query(
            r#"
            UPDATE jobs
            SET status = 'processing', 
                locked_at = CURRENT_TIMESTAMP, 
                locked_by = ?
            WHERE id = (
                SELECT id FROM jobs
                WHERE status = 'pending' AND run_at <= CURRENT_TIMESTAMP
                ORDER BY priority DESC, created_at ASC
                LIMIT 1
            )
            RETURNING id, job_type, payload, run_at, created_at, retries, last_error
            "#
        )
        .bind(worker_id)
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Backend(e.to_string()))?;

        if let Some(row) = row {
            use sqlx::Row;
            use chrono::NaiveDateTime;

            let id_str: String = row.try_get("id").map_err(|e| QueueError::Backend(e.to_string()))?;
            let id = Uuid::parse_str(&id_str).map_err(|_| QueueError::Backend("Invalid UUID".into()))?;
            let job_type: String = row.try_get("job_type").map_err(|e| QueueError::Backend(e.to_string()))?;
            let payload: Value = row.try_get("payload").map_err(|e| QueueError::Backend(e.to_string()))?;
            
            let run_at_naive: NaiveDateTime = row.try_get("run_at").map_err(|e| QueueError::Backend(e.to_string()))?;
            let created_at_naive: NaiveDateTime = row.try_get("created_at").map_err(|e| QueueError::Backend(e.to_string()))?;
            
            let retries: i64 = row.try_get("retries").unwrap_or(0);
            let last_error: Option<String> = row.try_get("last_error").ok();

            Ok(Some(JobEntry {
                id,
                job_type,
                payload,
                status: JobStatus::Running,
                created_at: created_at_naive.and_utc(),
                run_at: run_at_naive.and_utc(),
                attempts: retries as u32,
                last_error, 
            }))
        } else {
            Ok(None)
        }
    }

    async fn update_status(&self, id: Uuid, status: JobStatus, error: Option<String>, delay_secs: Option<u64>) -> Result<(), QueueError> {
        let status_str = match status {
            JobStatus::Completed => "completed",
            JobStatus::Failed(_) => "failed",
            JobStatus::Running => "processing",
            JobStatus::Pending => "pending",
            JobStatus::DeadLetter => "dead_letter",
        };

        if let JobStatus::Failed(_) = status {
            // Calculate run_at for retry with delay
            let delay = delay_secs.unwrap_or(60); // Default 60 second delay
            sqlx::query(
                r#"
                UPDATE jobs 
                SET status = 'pending', last_error = ?, locked_at = NULL, locked_by = NULL, 
                    retries = retries + 1, run_at = datetime('now', '+' || ? || ' seconds')
                WHERE id = ?
                "#
            )
            .bind(error)
            .bind(delay as i64)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;
        } else {
            sqlx::query(
                r#"
                UPDATE jobs 
                SET status = ?, last_error = ?, locked_at = NULL, locked_by = NULL
                WHERE id = ?
                "#
            )
            .bind(status_str)
            .bind(error)
            .bind(id.to_string())
            .execute(&self.pool)
            .await
            .map_err(|e| QueueError::Backend(e.to_string()))?;
        }

        Ok(())
    }

    async fn get_status(&self, id: Uuid) -> Result<JobStatus, QueueError> {
        use sqlx::Row;
        let row = sqlx::query(
            "SELECT status, retries FROM jobs WHERE id = ?"
        )
        .bind(id.to_string())
        .fetch_optional(&self.pool)
        .await
        .map_err(|e| QueueError::Backend(e.to_string()))?;

        match row {
            Some(r) => {
                let status_str: String = r.try_get("status").map_err(|e| QueueError::Backend(e.to_string()))?;
                let retries: i64 = r.try_get("retries").unwrap_or(0);
                
                match status_str.as_str() {
                    "pending" => Ok(JobStatus::Pending),
                    "processing" => Ok(JobStatus::Running),
                    "completed" => Ok(JobStatus::Completed),
                    "failed" => Ok(JobStatus::Failed(retries as u32)),
                    "dead_letter" => Ok(JobStatus::DeadLetter),
                    _ => Err(QueueError::Backend("Invalid status in DB".into())),
                }
            },
            None => Err(QueueError::NotFound),
        }
    }
}
