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
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        let data = self.embeddings.read().unwrap();
        let mut scores: Vec<(f32, VectorEmbedding)> = data
            .iter()
            .filter(|(_, tid, _)| tid == tenant_id)
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
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        // In a real high-perf vector DB we'd use HNSW/IVF.
        // For VEX P2, we perform a brute-force scan of the tenant's embeddings.
        let rows =
            sqlx::query("SELECT id, vector, metadata FROM vector_embeddings WHERE tenant_id = ?")
                .bind(tenant_id)
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
