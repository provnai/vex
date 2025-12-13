//! Temporal compression strategies

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

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
            Self::Step => if ratio < 0.5 { 1.0 } else { 0.3 },
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
    /// Use `compress_with_llm` for intelligent summarization
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

    /// Summarize content using an LLM for intelligent compression
    /// 
    /// # Arguments
    /// * `content` - The text to compress
    /// * `ratio` - Compression ratio (0.0 = no compression, 0.9 = 90% reduction)
    /// * `llm` - Any LlmProvider implementation
    /// 
    /// # Returns
    /// A summarized version of the content preserving key information
    pub async fn compress_with_llm<L: vex_llm::LlmProvider>(
        &self, 
        content: &str, 
        ratio: f64,
        llm: &L
    ) -> Result<String, vex_llm::LlmError> {
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

        // Use the LlmProvider's ask method for simple completion
        let summary = llm.ask(&prompt).await?;

        tracing::debug!(
            original_words = word_count,
            target_words = target_words,
            summary_words = summary.split_whitespace().count(),
            "LLM compression complete"
        );

        Ok(summary.trim().to_string())
    }

    /// Compress with fallback - tries LLM first, falls back to truncation
    pub async fn compress_smart<L: vex_llm::LlmProvider>(
        &self,
        content: &str,
        ratio: f64,
        llm: Option<&L>
    ) -> String {
        match llm {
            Some(provider) => {
                match self.compress_with_llm(content, ratio, provider).await {
                    Ok(summary) => summary,
                    Err(e) => {
                        tracing::warn!("LLM compression failed, using truncation: {}", e);
                        self.compress(content, ratio)
                    }
                }
            }
            None => self.compress(content, ratio)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decay_strategies() {
        let hour = Duration::hours(1);
        let half_hour = Duration::minutes(30);

        // Linear should be 0.5 at halfway point
        let linear = DecayStrategy::Linear.calculate(half_hour, hour);
        assert!((linear - 0.5).abs() < 0.01);

        // Step should be 1.0 before threshold
        let step = DecayStrategy::Step.calculate(Duration::minutes(20), hour);
        assert!((step - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_compressor() {
        let compressor = TemporalCompressor::default();
        let now = Utc::now();

        // Fresh content should have high importance
        let importance = compressor.importance(now, 1.0);
        assert!(importance > 0.9);

        // Should not evict fresh content
        assert!(!compressor.should_evict(now));
    }
}
