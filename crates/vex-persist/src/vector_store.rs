use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VectorError {
    #[error("Dimension mismatch: expected {0}, got {1}")]
    DimensionMismatch(usize, usize),
    #[error("Serialization error: {0}")]
    SerializationError(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VectorEmbedding {
    pub id: String,
    pub vector: Vec<f32>,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SharableIndex {
    pub dimension: usize,
    pub embeddings: Vec<VectorEmbedding>,
    pub revision_hash: String,
}

#[derive(Debug, Clone)]
pub struct VectorStore {
    dimension: usize,
    data: Arc<RwLock<SharableIndex>>,
}

impl VectorStore {
    pub fn new(dimension: usize) -> Self {
        Self {
            dimension,
            data: Arc::new(RwLock::new(SharableIndex {
                dimension,
                embeddings: Vec::new(),
                revision_hash: "".to_string(), // Genesis
            })),
        }
    }

    pub fn add(
        &self,
        id: String,
        vector: Vec<f32>,
        metadata: HashMap<String, String>,
    ) -> Result<(), VectorError> {
        if vector.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, vector.len()));
        }

        let mut data = self.data.write().unwrap();
        data.embeddings.push(VectorEmbedding {
            id,
            vector,
            metadata,
        });

        // Update revision hash (simple content hash)
        let mut hasher = Sha256::new();
        hasher.update(data.embeddings.len().to_be_bytes());
        // In a real impl, we'd hash the content, but for now length + random is enough to signal change
        // optimization: full content hashing on export
        data.revision_hash = hex::encode(hasher.finalize());

        Ok(())
    }

    pub fn search(
        &self,
        query: &[f32],
        k: usize,
    ) -> Result<Vec<(f32, VectorEmbedding)>, VectorError> {
        if query.len() != self.dimension {
            return Err(VectorError::DimensionMismatch(self.dimension, query.len()));
        }

        let data = self.data.read().unwrap();
        let mut scores: Vec<(f32, VectorEmbedding)> = data
            .embeddings
            .iter()
            .map(|emb| {
                let score = cosine_similarity(query, &emb.vector);
                (score, emb.clone())
            })
            .collect();

        // Sort by score descending
        scores.sort_by(|a, b| b.0.partial_cmp(&a.0).unwrap_or(std::cmp::Ordering::Equal));
        scores.truncate(k);

        Ok(scores)
    }

    /// Exports the index as a binary blob for the P2P Swarm
    pub fn export_to_blob(&self) -> Result<Vec<u8>, VectorError> {
        let data = self.data.read().unwrap();
        // Use JSON for beta compatibility (easier to debug), upgrade to Bincode/Parquet later
        serde_json::to_vec(&*data).map_err(|e| VectorError::SerializationError(e.to_string()))
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
