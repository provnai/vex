//! PostgreSQL storage backend implementation

use async_trait::async_trait;
use sqlx::postgres::{PgConnectOptions, PgPool, PgPoolOptions};
use std::str::FromStr;
use tracing::info;

use crate::backend::{StorageBackend, StorageError};

/// PostgreSQL configuration options
#[derive(Debug, Clone)]
pub struct PostgresConfig {
    /// Database URL (e.g., "postgres://user:pass@host/db")
    pub url: String,
    /// Maximum number of connections in the pool
    pub max_connections: u32,
    /// Minimum number of connections in the pool
    pub min_connections: u32,
    /// SSL mode enforcement
    pub ssl_required: bool,
}

impl Default for PostgresConfig {
    fn default() -> Self {
        Self {
            url: "postgres://localhost/vex".to_string(),
            max_connections: 20,
            min_connections: 2,
            ssl_required: false,
        }
    }
}

impl PostgresConfig {
    /// Create config from a DATABASE_URL string
    pub fn from_url(url: &str) -> Self {
        Self {
            url: url.to_string(),
            ..Default::default()
        }
    }

    /// Production config: higher pool, SSL required
    pub fn production(url: &str) -> Self {
        Self {
            url: url.to_string(),
            max_connections: 50,
            min_connections: 5,
            ssl_required: true,
        }
    }
}

/// PostgreSQL storage backend
#[derive(Debug)]
pub struct PostgresBackend {
    pool: PgPool,
}

impl PostgresBackend {
    /// Create a new PostgreSQL backend from a DATABASE_URL
    pub async fn new(url: &str) -> Result<Self, StorageError> {
        let config = PostgresConfig::from_url(url);
        Self::new_with_config(config).await
    }

    /// Create a new PostgreSQL backend with full configuration
    pub async fn new_with_config(config: PostgresConfig) -> Result<Self, StorageError> {
        let options = PgConnectOptions::from_str(&config.url)
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        let pool = PgPoolOptions::new()
            .max_connections(config.max_connections)
            .min_connections(config.min_connections)
            .connect_with(options)
            .await
            .map_err(|e| StorageError::Connection(e.to_string()))?;

        info!(
            url = %config.url,
            max_connections = config.max_connections,
            "Connected to PostgreSQL"
        );

        // Run migrations
        sqlx::migrate!("./postgres_migrations")
            .run(&pool)
            .await
            .map_err(|e| StorageError::Internal(format!("Migration failed: {}", e)))?;

        Ok(Self { pool })
    }

    /// Get the connection pool
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }
}

#[async_trait]
impl StorageBackend for PostgresBackend {
    fn name(&self) -> &str {
        "postgres"
    }

    async fn is_healthy(&self) -> bool {
        !self.pool.is_closed()
    }

    async fn set_value(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError> {
        let json = serde_json::to_string(&value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;

        let now = chrono::Utc::now().timestamp();

        // Postgres upsert using ON CONFLICT
        sqlx::query(
            "INSERT INTO kv_store (key, value, created_at, updated_at) VALUES ($1, $2, $3, $4)
             ON CONFLICT (key) DO UPDATE SET value = EXCLUDED.value, updated_at = EXCLUDED.updated_at"
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
        let result = sqlx::query("SELECT value FROM kv_store WHERE key = $1")
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
        let result = sqlx::query("DELETE FROM kv_store WHERE key = $1")
            .bind(key)
            .execute(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(result.rows_affected() > 0)
    }

    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        let result = sqlx::query("SELECT 1 FROM kv_store WHERE key = $1")
            .bind(key)
            .fetch_optional(&self.pool)
            .await
            .map_err(|e| StorageError::Query(e.to_string()))?;

        Ok(result.is_some())
    }

    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        use sqlx::Row;
        let pattern = format!("{}%", prefix);
        let rows = sqlx::query("SELECT key FROM kv_store WHERE key LIKE $1")
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
