//! SQLite backend implementation

use async_trait::async_trait;
use sqlx::sqlite::{SqliteConnectOptions, SqlitePool, SqlitePoolOptions};
use std::str::FromStr;
use tracing::{info, warn};

use crate::backend::{StorageBackend, StorageError};

/// SQLite configuration options
#[derive(Debug, Clone)]
pub struct SqliteConfig {
    /// Database URL (e.g., "sqlite:data.db" or "sqlite::memory:")
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Encryption key for SQLCipher (None = unencrypted)
    /// Note: Requires SQLite compiled with SQLCipher extension
    pub encryption_key: Option<String>,
    /// Enable WAL journal mode for better concurrency
    pub wal_mode: bool,
    /// Enable foreign key enforcement
    pub foreign_keys: bool,
    /// Busy timeout in seconds
    pub busy_timeout_secs: u32,
}

impl Default for SqliteConfig {
    fn default() -> Self {
        Self {
            url: "sqlite:vex.db?mode=rwc".to_string(),
            max_connections: 5,
            encryption_key: None,
            wal_mode: true,
            foreign_keys: true,
            busy_timeout_secs: 30,
        }
    }
}

impl SqliteConfig {
    /// Create config for in-memory database (testing)
    pub fn memory() -> Self {
        Self {
            url: "sqlite::memory:".to_string(),
            max_connections: 1,
            encryption_key: None,
            wal_mode: false,
            foreign_keys: true,
            busy_timeout_secs: 5,
        }
    }

    /// Create secure config with encryption
    pub fn secure(url: &str, encryption_key: &str) -> Self {
        Self {
            url: url.to_string(),
            max_connections: 5,
            encryption_key: Some(encryption_key.to_string()),
            wal_mode: true,
            foreign_keys: true,
            busy_timeout_secs: 30,
        }
    }
}

/// SQLite storage backend
#[derive(Debug)]
pub struct SqliteBackend {
    pool: SqlitePool,
    encrypted: bool,
}

impl SqliteBackend {
    /// Create a new SQLite backend with default config
    pub async fn new(url: &str) -> Result<Self, StorageError> {
        let config = SqliteConfig {
            url: url.to_string(),
            ..Default::default()
        };
        Self::new_with_config(config).await
    }

    /// Create a new SQLite backend with full configuration
    pub async fn new_with_config(config: SqliteConfig) -> Result<Self, StorageError> {
        let mut options = SqliteConnectOptions::from_str(&config.url)
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        // Set pragmas for security and performance
        if config.foreign_keys {
            options = options.pragma("foreign_keys", "ON");
        }
        options = options.pragma("busy_timeout", config.busy_timeout_secs.to_string());

        if config.wal_mode {
            options = options.pragma("journal_mode", "WAL");
        }

        // Handle encryption key (requires SQLCipher)
        let encrypted = if let Some(ref key) = config.encryption_key {
            // SQLCipher pragma - will fail silently if not compiled with SQLCipher
            options = options.pragma("key", format!("'{}'", key));
            warn!("SQLite encryption enabled - ensure SQLCipher is available");
            true
        } else {
            false
        };

        let pool = SqlitePoolOptions::new()
            .max_connections(config.max_connections)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        info!(
            url = %config.url,
            encrypted = encrypted,
            wal = config.wal_mode,
            "Connected to SQLite"
        );

        // Verify SQLCipher is actually active if encryption was requested
        if encrypted {
            use sqlx::Row;
            let _result = sqlx::query("SELECT sqlite3_version()")
                .fetch_one(&pool)
                .await
                .map_err(|e| StorageError::Connection(format!("SQLCipher verification failed: {}", e)))?;
            
            // Try to verify cipher_version pragma - if it fails, SQLCipher is not available
            let cipher_check = sqlx::query("PRAGMA cipher_version")
                .fetch_optional(&pool)
                .await;
            
            match cipher_check {
                Ok(Some(row)) => {
                    let version: Option<String> = row.try_get(0).ok();
                    if version.is_none() || version.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
                        return Err(StorageError::Internal(
                            "SQLCipher encryption requested but cipher_version returned empty. \
                             SQLite may not be compiled with SQLCipher support.".to_string()
                        ));
                    }
                    info!(cipher_version = ?version, "SQLCipher encryption verified");
                }
                Ok(None) | Err(_) => {
                    return Err(StorageError::Internal(
                        "SQLCipher encryption requested but not available. \
                         Database will NOT be encrypted! Aborting for security.".to_string()
                    ));
                }
            }
        }

        // Run migrations
        sqlx::migrate!("./migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Internal(format!("Migration failed: {}", e)))?;

        Ok(Self { pool, encrypted })
    }

    /// Get the connection pool
    pub fn pool(&self) -> &SqlitePool {
        &self.pool
    }

    /// Check if database is encrypted
    pub fn is_encrypted(&self) -> bool {
        self.encrypted
    }
}

#[async_trait]
impl StorageBackend for SqliteBackend {
    fn name(&self) -> &str {
        "sqlite"
    }

    async fn is_healthy(&self) -> bool {
        !self.pool.is_closed()
    }

    async fn set_value(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError> {
        let json = serde_json::to_string(&value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        sqlx::query(
            "INSERT OR REPLACE INTO kv_store (key, value, created_at, updated_at) VALUES (?, ?, ?, ?)"
        )
        .bind(key)
        .bind(json)
        .bind(now)
        .bind(now)
        .execute(&self.pool)
        .await
        .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(())
    }

    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError> {
        use sqlx::Row;
        let result = sqlx::query("SELECT value FROM kv_store WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        match result {
            Some(row) => {
                let value_str: String = row
                    .try_get("value")
                    .map_err(|e| StorageError::Query(e.to_string()))?;
                let value = serde_json::from_str(&value_str)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }

    async fn delete(&self, key: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("DELETE FROM kv_store WHERE key = ?")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("SELECT 1 FROM kv_store WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        use sqlx::Row;
        let pattern = format!("{}%", prefix);
        let rows = sqlx::query("SELECT key FROM kv_store WHERE key LIKE ?")
            .bind(pattern)
            .fetch_all(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        let mut keys = Vec::new();
        for row in rows {
            let key: String = row
                .try_get("key")
                .map_err(|e| StorageError::Query(e.to_string()))?;
            keys.push(key);
        }
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::StorageExt;
    use serde::{Deserialize, Serialize};

    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_sqlite_backend() {
        let backend = SqliteBackend::new("sqlite::memory:").await.unwrap();

        let data = TestData {
            name: "test_sql".to_string(),
            value: 99,
        };

        // Set
        backend.set("sql:1", &data).await.unwrap();

        // Exists
        assert!(backend.exists("sql:1").await.unwrap());

        // Get
        let retrieved: Option<TestData> = backend.get("sql:1").await.unwrap();
        assert_eq!(retrieved, Some(data));

        // List
        let keys = backend.list_keys("sql:").await.unwrap();
        assert_eq!(keys, vec!["sql:1"]);

        // Delete
        assert!(backend.delete("sql:1").await.unwrap());
        assert!(!backend.exists("sql:1").await.unwrap());
    }
}
