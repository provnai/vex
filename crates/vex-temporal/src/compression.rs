//! Temporal compression strategies

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use vex_llm::{EmbeddingProvider, LlmError, LlmProvider};
use vex_persist::VectorStoreBackend;

/// Strategy for decaying old context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DecayStrategy {
    /// Linear decay: importance decreases linearly with age
    Linear,
    /// Exponential decay: importance drops rapidly then stabilizes
    Exponential,
    /// Step decay: full importance until threshold, then compressed
    Step,
    /// None: no decay, manual control only
    None,
}

impl DecayStrategy {
    /// Calculate decay factor for given age
    /// Returns 1.0 for fresh, 0.0 for fully decayed
    pub fn calculate(&self, age: Duration, max_age: Duration) -> f64 {
        if max_age.num_seconds() == 0 {
            return 1.0;
        }

        let ratio = age.num_seconds() as f64 / max_age.num_seconds() as f64;
        let ratio = ratio.clamp(0.0, 1.0);

        match self {
            Self::Linear => 1.0 - ratio,
            Self::Exponential => (-3.0 * ratio).exp(),
            Self::Step => {
                if ratio < 0.5 {
                    1.0
                } else {
                    0.3
                }
            }
            Self::None => 1.0,
        }
    }
}

/// Compressor for temporal context
#[derive(Debug, Clone)]
pub struct TemporalCompressor {
    /// Decay strategy
    pub strategy: DecayStrategy,
    /// Maximum age before full decay
    pub max_age: Duration,
    /// Minimum importance threshold
    pub min_importance: f64,
}

impl Default for TemporalCompressor {
    fn default() -> Self {
        Self {
            strategy: DecayStrategy::Exponential,
            max_age: Duration::hours(24),
            min_importance: 0.1,
        }
    }
}

impl TemporalCompressor {
    /// Create a new compressor with given strategy
    pub fn new(strategy: DecayStrategy, max_age: Duration) -> Self {
        Self {
            strategy,
            max_age,
            min_importance: 0.1,
        }
    }

    /// Calculate current importance of content with given timestamp
    pub fn importance(&self, created_at: DateTime<Utc>, base_importance: f64) -> f64 {
        let age = Utc::now() - created_at;
        let decay = self.strategy.calculate(age, self.max_age);
        (base_importance * decay).max(self.min_importance)
    }

    /// Check if content should be evicted
    pub fn should_evict(&self, created_at: DateTime<Utc>) -> bool {
        let age = Utc::now() - created_at;
        age > self.max_age
    }

    /// Calculate compression ratio based on age
    pub fn compression_ratio(&self, created_at: DateTime<Utc>) -> f64 {
        let age = Utc::now() - created_at;
        let ratio = age.num_seconds() as f64 / self.max_age.num_seconds() as f64;
        ratio.clamp(0.0, 0.9) // Max 90% compression
    }

    /// Summarize content based on compression ratio (sync fallback - just truncates)
    pub fn compress(&self, content: &str, ratio: f64) -> String {
        if ratio <= 0.0 {
            return content.to_string();
        }

        let target_len = ((1.0 - ratio) * content.len() as f64) as usize;
        let target_len = target_len.max(20);

        if target_len >= content.len() {
            content.to_string()
        } else {
            format!("{}...[compressed]", &content[..target_len])
        }
    }

    /// Summarize content using an LLM and store it in semantic memory
    ///
    /// # Arguments
    /// * `content` - The text to compress
    /// * `ratio` - Compression ratio
    /// * `llm` - LLM and Embedding provider
    /// * `vector_store` - Optional persistent vector store for RAG fallback
    /// * `tenant_id` - Tenant ID for vector storage
    pub async fn compress_with_llm<L: LlmProvider + EmbeddingProvider>(
        &self,
        content: &str,
        ratio: f64,
        llm: &L,
        vector_store: Option<&dyn VectorStoreBackend>,
        tenant_id: Option<&str>,
    ) -> Result<String, LlmError> {
        // If no compression needed, return as-is
        if ratio <= 0.0 || content.len() < 50 {
            return Ok(content.to_string());
        }

        // Calculate target length
        let word_count = content.split_whitespace().count();
        let target_words = ((1.0 - ratio) * word_count as f64).max(10.0) as usize;

        // Build summarization prompt
        let prompt = format!(
            "Summarize the following text in approximately {} words. \
             Preserve the most important facts, decisions, and context. \
             Be concise but maintain accuracy.\n\n\
             TEXT TO SUMMARIZE:\n{}\n\n\
             SUMMARY:",
            target_words, content
        );

        let summary = llm.ask(&prompt).await?;

        // Semantic Memory Integration: Embed and store the summary
        if let (Some(vs), Some(tid)) = (vector_store, tenant_id) {
            match llm.embed(&summary).await {
                Ok(vector) => {
                    let mut metadata = HashMap::new();
                    metadata.insert("type".to_string(), "temporal_summary".to_string());
                    metadata.insert("original_len".to_string(), content.len().to_string());
                    metadata.insert("timestamp".to_string(), Utc::now().to_rfc3339());

                    let id = format!("summary_{}", uuid::Uuid::new_v4());
                    if let Err(e) = vs.add(id, tid.to_string(), vector, metadata).await {
                        tracing::warn!("Failed to store summary embedding: {}", e);
                    }
                }
                Err(e) => tracing::warn!("Failed to generate summary embedding: {}", e),
            }
        }

        Ok(summary.trim().to_string())
    }
}
