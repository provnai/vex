//! Episodic memory management

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

use crate::compression::TemporalCompressor;
use crate::horizon::HorizonConfig;

/// An episode in memory
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Episode {
    /// Unique identifier
    pub id: u64,
    /// Content of the episode
    pub content: String,
    /// When it was created
    pub created_at: DateTime<Utc>,
    /// Base importance (0.0 - 1.0)
    pub base_importance: f64,
    /// Whether this episode is pinned (never evicted)
    pub pinned: bool,
    /// Tags for categorization
    pub tags: Vec<String>,
}

impl Episode {
    /// Create a new episode
    pub fn new(content: &str, importance: f64) -> Self {
        static COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
        Self {
            id: COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed),
            content: content.to_string(),
            created_at: Utc::now(),
            base_importance: importance.clamp(0.0, 1.0),
            pinned: false,
            tags: Vec::new(),
        }
    }

    /// Create a pinned episode (never evicted)
    pub fn pinned(content: &str) -> Self {
        let mut ep = Self::new(content, 1.0);
        ep.pinned = true;
        ep
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: &str) -> Self {
        self.tags.push(tag.to_string());
        self
    }
}

/// Episodic memory store
#[derive(Debug, Clone)]
pub struct EpisodicMemory {
    /// Configuration
    pub config: HorizonConfig,
    /// Compressor
    pub compressor: TemporalCompressor,
    /// Episodes (most recent first)
    episodes: VecDeque<Episode>,
}

impl EpisodicMemory {
    /// Create new episodic memory with config
    pub fn new(config: HorizonConfig) -> Self {
        let max_age = config
            .horizon
            .duration()
            .unwrap_or(chrono::Duration::weeks(52));
        Self {
            config,
            compressor: TemporalCompressor::new(
                crate::compression::DecayStrategy::Exponential,
                max_age,
            ),
            episodes: VecDeque::new(),
        }
    }

    /// Add a new episode
    pub fn add(&mut self, episode: Episode) {
        self.episodes.push_front(episode);
        self.maybe_evict();
    }

    /// Add simple content
    pub fn remember(&mut self, content: &str, importance: f64) {
        self.add(Episode::new(content, importance));
    }

    /// Get all episodes
    pub fn episodes(&self) -> impl Iterator<Item = &Episode> {
        self.episodes.iter()
    }

    /// Get episodes by tag
    pub fn by_tag(&self, tag: &str) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|e| e.tags.contains(&tag.to_string()))
            .collect()
    }

    /// Get recent episodes (within horizon)
    pub fn recent(&self) -> Vec<&Episode> {
        self.episodes
            .iter()
            .filter(|e| self.config.horizon.contains(e.created_at))
            .collect()
    }

    /// Get episodes sorted by current importance
    pub fn by_importance(&self) -> Vec<(&Episode, f64)> {
        let mut episodes: Vec<_> = self
            .episodes
            .iter()
            .map(|e| {
                let importance = if e.pinned {
                    1.0
                } else {
                    self.compressor.importance(e.created_at, e.base_importance)
                };
                (e, importance)
            })
            .collect();
        episodes.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap());
        episodes
    }

    /// Get total episode count
    pub fn len(&self) -> usize {
        self.episodes.len()
    }

    /// Check if empty
    pub fn is_empty(&self) -> bool {
        self.episodes.is_empty()
    }

    /// Clear all non-pinned episodes
    pub fn clear(&mut self) {
        self.episodes.retain(|e| e.pinned);
    }

    /// Evict old episodes if over capacity
    fn maybe_evict(&mut self) {
        if !self.config.auto_evict {
            return;
        }

        // Evict by age first
        self.episodes
            .retain(|e| e.pinned || !self.compressor.should_evict(e.created_at));

        // Then evict by count
        while self.episodes.len() > self.config.max_entries {
            // Find least important non-pinned episode
            let maybe_idx = self
                .episodes
                .iter()
                .enumerate()
                .filter(|(_, e)| !e.pinned)
                .min_by(|(_, a), (_, b)| {
                    let imp_a = self.compressor.importance(a.created_at, a.base_importance);
                    let imp_b = self.compressor.importance(b.created_at, b.base_importance);
                    imp_a.partial_cmp(&imp_b).unwrap()
                })
                .map(|(i, _)| i);

            if let Some(idx) = maybe_idx {
                self.episodes.remove(idx);
            } else {
                break; // All pinned
            }
        }
    }

    /// Compress old episodes (returns number compressed) - sync fallback using truncation
    pub fn compress_old(&mut self) -> usize {
        if !self.config.auto_compress {
            return 0;
        }

        let mut count = 0;
        for episode in &mut self.episodes {
            if episode.pinned {
                continue;
            }

            let ratio = self.compressor.compression_ratio(episode.created_at);
            if ratio > 0.1 {
                episode.content = self.compressor.compress(&episode.content, ratio);
                count += 1;
            }
        }
        count
    }

    /// Compress old episodes using LLM for intelligent summarization
    /// Returns the number of episodes that were compressed
    pub async fn compress_old_with_llm<L: vex_llm::LlmProvider>(&mut self, llm: &L) -> usize {
        if !self.config.auto_compress {
            return 0;
        }

        let mut count = 0;
        for episode in &mut self.episodes {
            if episode.pinned {
                continue;
            }

            let ratio = self.compressor.compression_ratio(episode.created_at);
            if ratio > 0.1 {
                match self
                    .compressor
                    .compress_with_llm(&episode.content, ratio, llm)
                    .await
                {
                    Ok(compressed) => {
                        tracing::debug!(
                            episode_id = %episode.id,
                            original_len = episode.content.len(),
                            compressed_len = compressed.len(),
                            ratio = ratio,
                            "Compressed episode with LLM"
                        );
                        episode.content = compressed;
                        count += 1;
                    }
                    Err(e) => {
                        tracing::warn!("LLM compression failed for episode {}: {}", episode.id, e);
                        // Fallback to truncation
                        episode.content = self.compressor.compress(&episode.content, ratio);
                        count += 1;
                    }
                }
            }
        }
        count
    }

    /// Summarize all episodes into a single context string using LLM
    /// Useful for providing memory context to agents
    pub async fn summarize_all_with_llm<L: vex_llm::LlmProvider>(
        &self,
        llm: &L,
    ) -> Result<String, vex_llm::LlmError> {
        if self.episodes.is_empty() {
            return Ok(String::from("No memories recorded."));
        }

        // Combine all episodes into a single text
        let all_content: String = self
            .episodes
            .iter()
            .map(|e| {
                format!(
                    "[{}] (importance: {:.1}): {}",
                    e.created_at.format("%Y-%m-%d %H:%M"),
                    e.base_importance,
                    e.content
                )
            })
            .collect::<Vec<_>>()
            .join("\n\n");

        let prompt = format!(
            "You are a memory consolidation system. Summarize the following episodic memories \
             into a coherent narrative that preserves the most important information, decisions, \
             and context. Focus on factual content and key events.\n\n\
             MEMORIES:\n{}\n\n\
             CONSOLIDATED SUMMARY:",
            all_content
        );

        llm.ask(&prompt).await.map(|s| s.trim().to_string())
    }

    /// Get a summary of memory contents
    pub fn summarize(&self) -> String {
        let total = self.len();
        let pinned = self.episodes.iter().filter(|e| e.pinned).count();
        let recent = self.recent().len();

        format!(
            "Memory: {} total ({} pinned, {} recent within {:?})",
            total, pinned, recent, self.config.horizon
        )
    }
}

impl Default for EpisodicMemory {
    fn default() -> Self {
        Self::new(HorizonConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_episodic_memory() {
        let mut memory = EpisodicMemory::default();

        memory.remember("First event", 0.8);
        memory.remember("Second event", 0.5);
        memory.add(Episode::pinned("Important system info"));

        assert_eq!(memory.len(), 3);
        assert_eq!(memory.recent().len(), 3);
    }

    #[test]
    fn test_by_importance() {
        let mut memory = EpisodicMemory::default();

        memory.remember("Low importance", 0.2);
        memory.remember("High importance", 0.9);

        let sorted = memory.by_importance();
        assert!(sorted[0].1 > sorted[1].1);
    }

    #[test]
    fn test_pinned_not_evicted() {
        let mut config = HorizonConfig::default();
        config.max_entries = 2;

        let mut memory = EpisodicMemory::new(config);
        memory.add(Episode::pinned("System"));
        memory.remember("Event 1", 0.5);
        memory.remember("Event 2", 0.5);
        memory.remember("Event 3", 0.5);

        // Pinned should still be there
        assert!(memory.episodes().any(|e| e.content == "System"));
    }
}
