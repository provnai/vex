use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Row, SqlitePool};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Dimension mismatch: expected {0}, got {1}")]
    DimensionMismatch(usize, usize),
    #[error("Serialization error: {0}")]
    SerializationError(String),
    #[error("Database error: {0}")]
    DatabaseError(String),
    #[error("Storage full: capacity exceeded")]
    StorageFull,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

/// Generic trait for vector storage
#[async_trait]
pub trait VectorStoreBackend: Send + Sync + std::fmt::Debug {
    async fn add(
        &self,
        id: String,
        tenant_id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VectorError>;

    async fn search(
        &self,
        tenant_id: &str,
        query: &[f32],
        k: usize,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError>;
}

/// In-memory vector store implementation (for testing and small contexts)
#[derive(Debug, Clone)]
pub struct MemoryVectorStore {
    dimension: usize,
    embeddings: Arc<RwLock<Vec<(String, String, VectorEmbedding)>>>, // (id, tenant_id, embedding)
}

impl MemoryVectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            embeddings: Arc::new(RwLock::new(Vec::new())),
        }
    }
}

#[async_trait]
impl VectorStoreBackend for MemoryVectorStore {
    async fn add(
        &self,
        id: String,
        tenant_id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VectorError> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, vector.len()));
        }

        let mut data = self.embeddings.write().unwrap();

        // Limit capacity to prevent memory DoS (Fix #12)
        if data.len() >= 100_000 {
            return Err(VectorError::StorageFull);
        }

        data.push((
            id.clone(),
            tenant_id,
            VectorEmbedding {
                id,
                vector,
                metadata,
            },
        ));

        Ok(())
    }

    async fn search(
        &self,
        tenant_id: &str,
        query: &[f32],
        k: usize,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        let data = self.embeddings.read().unwrap();
        let mut scores: Vec<(f32, VectorEmbedding)> = data
            .iter()
            .filter(|(_, tid, emb)| {
                if tid != tenant_id {
                    return false;
                }

                // Apply metadata filters
                if let Some(ref f) = filters {
                    for (key, val) in f {
                        if emb.metadata.get(key) != Some(val) {
                            return false;
                        }
                    }
                }

                true
            })
            .map(|(_, _, emb)| {
                let score = cosine_similarity(query, &emb.vector);
                (score, emb.clone())
            })
            .collect();

        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);

        Ok(scores)
    }
}

/// SQLite-backed persistent vector store
#[derive(Debug, Clone)]
pub struct SqliteVectorStore {
    dimension: usize,
    pool: SqlitePool,
}

impl SqliteVectorStore {
    pub fn new(dimension: usize, pool: SqlitePool) -> Self {
        Self { dimension, pool }
    }
}

#[async_trait]
impl VectorStoreBackend for SqliteVectorStore {
    async fn add(
        &self,
        id: String,
        tenant_id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VectorError> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, vector.len()));
        }

        // Convert f32 vector to bytes (Little Endian)
        let mut vector_bytes = Vec::with_capacity(vector.len() * 4);
        for &val in &vector {
            vector_bytes.extend_from_slice(&val.to_le_bytes());
        }

        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| VectorError::SerializationError(e.to_string()))?;

        sqlx::query(
            "INSERT OR REPLACE INTO vector_embeddings (id, tenant_id, vector, metadata, created_at) VALUES (?, ?, ?, ?, ?)"
        )
        .bind(id)
        .bind(tenant_id)
        .bind(vector_bytes)
        .bind(metadata_json)
        .bind(chrono::Utc::now().timestamp())
        .execute(&self.pool)
        .await
        .map_err(|e| VectorError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn search(
        &self,
        tenant_id: &str,
        query: &[f32],
        k: usize,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        let mut sql =
            "SELECT id, vector, metadata FROM vector_embeddings WHERE tenant_id = ?".to_string();
        if let Some(ref f) = filters {
            for key in f.keys() {
                sql.push_str(&format!(" AND json_extract(metadata, '$.{}') = ?", key));
            }
        }

        let mut q = sqlx::query(&sql).bind(tenant_id);

        if let Some(ref f) = filters {
            for val in f.values() {
                q = q.bind(val);
            }
        }

        let rows = q
            .fetch_all(&self.pool)
            .await
            .map_err(|e| VectorError::DatabaseError(e.to_string()))?;

        let mut scores = Vec::new();

        for row in rows {
            let id: String = row.get("id");
            let vector_bytes: Vec<u8> = row.get("vector");
            let metadata_str: String = row.get("metadata");

            // Convert bytes back to f32 vector
            if vector_bytes.len() != self.dimension * 4 {
                continue; // Skip corrupted entry
            }

            let mut vector = Vec::with_capacity(self.dimension);
            for chunk in vector_bytes.chunks_exact(4) {
                let arr: [u8; 4] = chunk.try_into().unwrap();
                vector.push(f32::from_le_bytes(arr));
            }

            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str)
                .map_err(|e| VectorError::SerializationError(e.to_string()))?;

            let score = cosine_similarity(query, &vector);
            scores.push((
                score,
                VectorEmbedding {
                    id,
                    vector,
                    metadata,
                },
            ));
        }

        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);

        Ok(scores)
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    let dot_product: f32 = a.iter().zip(b).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();

    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }

    dot_product / (norm_a * norm_b)
}

/// PostgreSQL-backed vector store using pgvector native extension
/// Uses HNSW index for fast approximate nearest-neighbor search
/// Requires the `vector` extension: `CREATE EXTENSION IF NOT EXISTS vector;`
#[cfg(feature = "postgres")]
#[derive(Debug, Clone)]
pub struct PgVectorStore {
    dimension: usize,
    pool: sqlx::PgPool,
}

#[cfg(feature = "postgres")]
impl PgVectorStore {
    pub fn new(dimension: usize, pool: sqlx::PgPool) -> Self {
        Self { dimension, pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait]
impl VectorStoreBackend for PgVectorStore {
    async fn add(
        &self,
        id: String,
        tenant_id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VectorError> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, vector.len()));
        }

        let metadata_json = serde_json::to_string(&metadata)
            .map_err(|e| VectorError::SerializationError(e.to_string()))?;

        // Use pgvector::Vector type for native Postgres vector storage
        let pg_vector = pgvector::Vector::from(vector);

        sqlx::query(
            "INSERT INTO vector_embeddings (id, tenant_id, vector, metadata) VALUES ($1, $2, $3::vector, $4)
             ON CONFLICT (id, tenant_id) DO UPDATE SET vector = EXCLUDED.vector, metadata = EXCLUDED.metadata"
        )
        .bind(&id)
        .bind(&tenant_id)
        .bind(pg_vector)
        .bind(metadata_json)
        .execute(&self.pool)
        .await
        .map_err(|e| VectorError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    async fn search(
        &self,
        tenant_id: &str,
        query: &[f32],
        k: usize,
        filters: Option<HashMap<String, String>>,
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        let pg_query = pgvector::Vector::from(query.to_vec());
        let filters_json = filters
            .as_ref()
            .map(|f| serde_json::to_string(f).unwrap_or_else(|_| "{}".to_string()));

        let rows = if let Some(fj) = filters_json {
            sqlx::query(
                "SELECT id, vector, metadata, 1 - (vector <=> $1::vector) AS score
                 FROM vector_embeddings
                 WHERE tenant_id = $2 AND metadata @> $3::jsonb
                 ORDER BY vector <=> $1::vector
                 LIMIT $4",
            )
            .bind(pg_query)
            .bind(tenant_id)
            .bind(fj)
            .bind(k as i64)
            .fetch_all(&self.pool)
            .await
        } else {
            sqlx::query(
                "SELECT id, vector, metadata, 1 - (vector <=> $1::vector) AS score
                 FROM vector_embeddings
                 WHERE tenant_id = $2
                 ORDER BY vector <=> $1::vector
                 LIMIT $3",
            )
            .bind(pg_query)
            .bind(tenant_id)
            .bind(k as i64)
            .fetch_all(&self.pool)
            .await
        }
        .map_err(|e| VectorError::DatabaseError(e.to_string()))?;

        let mut results = Vec::new();
        for row in rows {
            use sqlx::Row;
            let id: String = row.get("id");
            let metadata_str: String = row.get("metadata");
            let score: f32 = row.try_get("score").unwrap_or(0.0);
            let pg_vec: pgvector::Vector = row.get("vector");
            let vector: Vec<f32> = pg_vec.to_vec();

            let metadata: HashMap<String, String> = serde_json::from_str(&metadata_str)
                .map_err(|e| VectorError::SerializationError(e.to_string()))?;

            results.push((
                score,
                VectorEmbedding {
                    id,
                    vector,
                    metadata,
                },
            ));
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_memory_vector_store_filtering() {
        let store = MemoryVectorStore::new(3);
        let tenant = "t1";

        let mut m1 = HashMap::new();
        m1.insert("type".to_string(), "a".to_string());
        m1.insert("cat".to_string(), "1".to_string());

        let mut m2 = HashMap::new();
        m2.insert("type".to_string(), "b".to_string());

        store
            .add("1".into(), tenant.into(), vec![1.0, 0.0, 0.0], m1)
            .await
            .unwrap();
        store
            .add("2".into(), tenant.into(), vec![0.0, 1.0, 0.0], m2)
            .await
            .unwrap();

        // 1. Filter by type=a
        let mut filter = HashMap::new();
        filter.insert("type".to_string(), "a".to_string());
        let results = store
            .search(tenant, &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "1");

        // 2. Filter by non-existent type
        let mut filter = HashMap::new();
        filter.insert("type".to_string(), "c".to_string());
        let results = store
            .search(tenant, &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 0);

        // 3. Multi-filter
        let mut filter = HashMap::new();
        filter.insert("type".to_string(), "a".to_string());
        filter.insert("cat".to_string(), "1".to_string());
        let results = store
            .search(tenant, &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "1");
    }

    #[tokio::test]
    async fn test_sqlite_vector_store_filtering() {
        let pool = SqlitePool::connect("sqlite::memory:").await.unwrap();

        // Setup table
        sqlx::query("CREATE TABLE vector_embeddings (id TEXT PRIMARY KEY, tenant_id TEXT NOT NULL, vector BLOB NOT NULL, metadata JSON NOT NULL, created_at INTEGER NOT NULL)")
            .execute(&pool).await.unwrap();

        let store = SqliteVectorStore::new(3, pool);
        let tenant = "t1";

        let mut m1 = HashMap::new();
        m1.insert("type".to_string(), "a".to_string());

        let mut m2 = HashMap::new();
        m2.insert("type".to_string(), "b".to_string());

        store
            .add("1".into(), tenant.into(), vec![1.0, 0.0, 0.0], m1)
            .await
            .unwrap();
        store
            .add("2".into(), tenant.into(), vec![0.0, 1.0, 0.0], m2)
            .await
            .unwrap();

        // 1. Filter by type=a
        let mut filter = HashMap::new();
        filter.insert("type".to_string(), "a".to_string());
        let results = store
            .search(tenant, &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].1.id, "1");

        // 2. Filter by non-existent type
        let mut filter = HashMap::new();
        filter.insert("type".to_string(), "c".to_string());
        let results = store
            .search(tenant, &[1.0, 0.0, 0.0], 10, Some(filter))
            .await
            .unwrap();
        assert_eq!(results.len(), 0);
    }
}
