use crate::error::AttestError;
use crate::persist::audit::{ActorType, AuditEvent, AuditEventType};
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::path::Path;
use std::str::FromStr;

pub struct LocalStore {
    pool: SqlitePool,
}

impl LocalStore {
    /// Open or create a new local SQLite store
    pub async fn new<P: AsRef<Path>>(path: P) -> Result<Self, AttestError> {
        let path_str = path
            .as_ref()
            .to_str()
            .ok_or_else(|| AttestError::Alignment("Invalid path".into()))?;

        let options = SqliteConnectOptions::from_str(&format!("sqlite:{}", path_str))
            .map_err(|e| AttestError::Alignment(format!("Bad DB URL: {}", e)))?
            .create_if_missing(true)
            .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal);

        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options)
            .await?;

        let store = Self { pool };
        store.init_schema().await?;
        Ok(store)
    }

    /// Initialize the ecosystem-compatible schema
    async fn init_schema(&self) -> Result<(), AttestError> {
        // Journal mode handled in connect options now

        sqlx::query(
            "CREATE TABLE IF NOT EXISTS audit_events (
                id TEXT PRIMARY KEY,
                event_type TEXT NOT NULL,
                timestamp INTEGER NOT NULL,
                agent_id TEXT,
                data TEXT NOT NULL,
                hash TEXT NOT NULL,
                previous_hash TEXT,
                sequence_number INTEGER NOT NULL,
                actor TEXT NOT NULL,
                rationale TEXT,
                signature TEXT,
                zk_proof TEXT
            )",
        )
        .execute(&self.pool)
        .await?;

        // Index for fast chain reconstruction
        sqlx::query(
            "CREATE INDEX IF NOT EXISTS idx_audit_sequence ON audit_events (sequence_number)",
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Save an audit event to the local database
    pub async fn save_event(&self, event: &AuditEvent) -> Result<(), AttestError> {
        sqlx::query(
            "INSERT OR REPLACE INTO audit_events            (
                id, event_type, timestamp, agent_id, data, hash, previous_hash, sequence_number, actor, rationale, signature, zk_proof
            ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)"
        )
        .bind(event.id.to_string())
        .bind(serde_json::to_string(&event.event_type).unwrap_or_default())
        .bind(event.timestamp.timestamp())
        .bind(event.agent_id.map(|id| id.to_string()))
        .bind(event.data.to_string())
        .bind(&event.hash)
        .bind(&event.previous_hash)
        .bind(event.sequence_number as i64)
        .bind(serde_json::to_string(&event.actor).unwrap_or_default())
        .bind(&event.rationale)
        .bind(&event.signature)
        .bind(&event.zk_proof)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Retrieve the last event hash for chaining
    pub async fn get_last_hash(&self) -> Result<Option<String>, AttestError> {
        let row: Option<(String,)> =
            sqlx::query_as("SELECT hash FROM audit_events ORDER BY sequence_number DESC LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;

        Ok(row.map(|r| r.0))
    }

    /// Retrieve the next sequence number
    pub async fn get_next_sequence(&self) -> Result<u64, AttestError> {
        let row: (Option<i64>,) = sqlx::query_as("SELECT MAX(sequence_number) FROM audit_events")
            .fetch_one(&self.pool)
            .await?;

        Ok(row.0.map(|v| (v + 1) as u64).unwrap_or(0))
    }

    /// Retrieve all events ordered by sequence number (for verification)
    pub async fn get_all_events(&self) -> Result<Vec<AuditEvent>, AttestError> {
        use sqlx::Row;

        let rows = sqlx::query(
            "SELECT 
                id,
                event_type,
                timestamp,
                agent_id,
                data,
                hash,
                previous_hash,
                sequence_number,
                actor,
                rationale,
                signature,
                zk_proof
            FROM audit_events ORDER BY sequence_number ASC",
        )
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::new();
        for row in rows {
            // Manual mapping to handle Type mismatches (i64 -> u64, String -> Enum)
            // Note: Enums were stored as Debug string formatting, which is slightly unsafe but we'll try to deserialize directly from JSON if we switch to that,
            // or we need to parse the Debug string.
            // Wait, save_event used `format!("{:?}")`. This is bad practice but exists.
            // However, `serde_json::from_str` won't work on Debug format unless Debug == JSON string.
            // Let's assume for verification we might need to parse.
            // Actually, for "Tamper Proofing", verifying the HASH is most important.
            // The compute_hash uses `format!("{:?}")` as well!
            // So as long as we reconstruct the struct fields exactly as they were when hashed, we are good.
            // BUT, `get_all_events` needs to return `AuditEvent`.
            // `AuditEventType` needs to be reconstructed.
            // Since we don't have a parser for the Debug format, we are slightly stuck unless we change storage to JSON.
            // Let's change storage to use JSON for Enums in `persist/sqlite.rs` AND `persist/audit.rs`.
            // This is "Tier 2 Cleanup" but necessary for correct verification.

            // For now, to unblock, let's implement a quick hack:
            // We can't easily parse `Debug` output in Rust.

            // REFACTOR: Update `save_event` to store Enums as JSON strings.
            // REFACTOR: Update `compute_hash` to use JSON strings.
            // Then we can `serde_json::from_str`.

            // But modifying `compute_hash` breaks chain compatibility with existing logs (if any).
            // Since this is a new rewrite, we can standardise on JSON now.

            // Let's defer that refactor and look at `AuditEvent`.
            // `AuditEventType` is simple enum. `ActorType` is simple.
            // Debug of `AgentCreated` is "AgentCreated".
            // JSON of `AgentCreated` is "AgentCreated" (if unit variant).
            // So serde_json might work for unit variants.
            // `Custom(String)` -> Debug: `Custom("foo")` vs JSON: `{"Custom": "foo"}`.

            // OK, to allow `verify_chain` to work properly with "no mock", we MUST ensure serialization stability.
            // I will update `save_event` to use `serde_json::to_string` instead of Format Debug.
            // And update `compute_hash` to use `serde_json::to_string`.

            let event_type_str: String = row.try_get("event_type")?;
            let actor_str: String = row.try_get("actor")?;
            let data_str: String = row.try_get("data")?; // Stored as TEXT

            // We'll try to deserialize from JSON. logic below assumes we fix the saver.
            let event_type: AuditEventType = serde_json::from_str(&event_type_str)
                .map_err(|e| AttestError::Alignment(format!("Bad event type json: {}", e)))?;
            let actor: ActorType = serde_json::from_str(&actor_str)
                .map_err(|e| AttestError::Alignment(format!("Bad actor json: {}", e)))?;
            let data: serde_json::Value = serde_json::from_str(&data_str)
                .map_err(|e| AttestError::Alignment(format!("Bad data json: {}", e)))?;

            let timestamp_int: i64 = row.try_get("timestamp")?;
            let sequence_int: i64 = row.try_get("sequence_number")?;
            let agent_id_str: Option<String> = row.try_get("agent_id")?;

            events.push(AuditEvent {
                id: uuid::Uuid::parse_str(&row.try_get::<String, _>("id")?).unwrap_or_default(),
                event_type,
                timestamp: chrono::DateTime::from_timestamp(timestamp_int, 0)
                    .ok_or_else(|| AttestError::Alignment("Bad timestamp".into()))?,
                agent_id: agent_id_str.map(|s| uuid::Uuid::parse_str(&s).unwrap_or_default()),
                data,
                hash: row.try_get("hash")?,
                previous_hash: row.try_get("previous_hash")?,
                sequence_number: sequence_int as u64,
                actor,
                rationale: row.try_get("rationale")?,
                signature: row.try_get("signature")?,
                zk_proof: row.try_get("zk_proof")?,
            });
        }

        Ok(events)
    }

    /// Explicitly close the connection pool (useful for tests or shutdown)
    pub async fn close(&self) {
        self.pool.close().await;
    }

    /// Execute raw SQL (DANGEROUS: For Admin/Test/Migration only)
    pub async fn execute_raw_sql(&self, sql: &str) -> Result<(), AttestError> {
        sqlx::query(sql).execute(&self.pool).await?;
        Ok(())
    }
}
