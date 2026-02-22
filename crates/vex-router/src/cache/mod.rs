//! Semantic Caching - Cache responses using vector embeddings

use serde::{Deserialize, Serialize};
use std::sync::Arc;
use parking_lot::RwLock;
use std::collections::HashMap;
use sha2::{Sha256, Digest};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedResponse {
    pub response: String,
    pub similarity: f32,
    pub cached_at: i64,
    pub token_count: u32,
}

#[derive(Debug, Clone)]
pub struct CacheEntry {
    pub response: CachedResponse,
    pub embedding: Vec<f32>,
}

pub struct SemanticCache {
    entries: Arc<RwLock<HashMap<String, CacheEntry>>>,
    similarity_threshold: f32,
    max_cache_size: usize,
    ttl_seconds: i64,
}

impl SemanticCache {
    pub fn new(similarity_threshold: f32, max_cache_size: usize, ttl_seconds: i64) -> Self {
        Self {
            entries: Arc::new(RwLock::new(HashMap::new())),
            similarity_threshold,
            max_cache_size,
            ttl_seconds,
        }
    }

    pub fn get(&self, query: &str) -> Option<CachedResponse> {
        let query_embedding = self.compute_embedding(query);
        let entries = self.entries.read();
        
        let mut best_match: Option<(f32, &CacheEntry)> = None;
        
        for (_key, entry) in entries.iter() {
            let similarity = cosine_similarity(&query_embedding, &entry.embedding);
            
            if similarity >= self.similarity_threshold 
                && (best_match.is_none() || similarity > best_match.as_ref().unwrap().0) {
                best_match = Some((similarity, entry));
            }
        }
        
        if let Some((similarity, entry)) = best_match {
            let now = chrono::Utc::now().timestamp();
            if now - entry.response.cached_at < self.ttl_seconds {
                let mut response = entry.response.clone();
                response.similarity = similarity;
                return Some(response);
            }
        }
        
        None
    }

    pub fn store(&self, query: &str, response: String, token_count: u32) {
        let key = self.compute_key(query);
        let embedding = self.compute_embedding(query);
        
        let mut entries = self.entries.write();
        
        if entries.len() >= self.max_cache_size {
            if let Some(oldest_key) = entries.iter()
                .min_by_key(|(_, e)| e.response.cached_at)
                .map(|(k, _)| k.clone())
            {
                entries.remove(&oldest_key);
            }
        }
        
        entries.insert(key, CacheEntry {
            response: CachedResponse {
                response,
                similarity: 1.0,
                cached_at: chrono::Utc::now().timestamp(),
                token_count,
            },
            embedding,
        });
    }

    fn compute_key(&self, query: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(query.as_bytes());
        hex::encode(hasher.finalize())
    }

    fn compute_embedding(&self, query: &str) -> Vec<f32> {
        simple_embedding(query)
    }

    pub fn stats(&self) -> CacheStats {
        let entries = self.entries.read();
        let now = chrono::Utc::now().timestamp();
        
        let valid_entries = entries.values()
            .filter(|e| now - e.response.cached_at < self.ttl_seconds)
            .count();
        
        CacheStats {
            total_entries: entries.len(),
            valid_entries,
            cache_size_bytes: entries.values()
                .map(|e| e.response.response.len() + e.embedding.len() * 4)
                .sum(),
        }
    }

    pub fn clear(&self) {
        let mut entries = self.entries.write();
        entries.clear();
    }
}

fn cosine_similarity(a: &[f32], b: &[f32]) -> f32 {
    if a.len() != b.len() || a.is_empty() {
        return 0.0;
    }
    
    let dot: f32 = a.iter().zip(b.iter()).map(|(x, y)| x * y).sum();
    let norm_a: f32 = a.iter().map(|x| x * x).sum::<f32>().sqrt();
    let norm_b: f32 = b.iter().map(|x| x * x).sum::<f32>().sqrt();
    
    if norm_a == 0.0 || norm_b == 0.0 {
        return 0.0;
    }
    
    dot / (norm_a * norm_b)
}

fn simple_embedding(text: &str) -> Vec<f32> {
    let text_lower = text.to_lowercase();
    let words: Vec<&str> = text_lower.split_whitespace().collect();
    
    let mut embedding = vec![0.0f32; 64];
    
    for (i, word) in words.iter().take(64).enumerate() {
        let hash = simple_hash(word);
        embedding[i % 64] += (hash as f32) / (words.len() as f32).sqrt();
    }
    
    let norm: f32 = embedding.iter().map(|x| x * x).sum::<f32>().sqrt();
    if norm > 0.0 {
        for x in &mut embedding {
            *x /= norm;
        }
    }
    
    embedding
}

fn simple_hash(s: &str) -> u32 {
    let mut hash: u32 = 5381;
    for c in s.bytes() {
        hash = hash.wrapping_mul(33).wrapping_add(c as u32);
    }
    hash
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub total_entries: usize,
    pub valid_entries: usize,
    pub cache_size_bytes: usize,
}

impl Default for SemanticCache {
    fn default() -> Self {
        Self::new(0.85, 10000, 86400)
    }
}
