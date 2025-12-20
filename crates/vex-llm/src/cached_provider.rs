//! Cached LLM provider wrapper using Moka
//!
//! Provides in-memory caching of LLM responses to reduce latency and API costs.
//! Uses the Moka cache library for high-performance concurrent caching with
//! TTL-based expiration.
//!
//! # 2025 Best Practices
//! - Uses SHA-256 hash of request as cache key
//! - Configurable TTL and max entries
//! - Thread-safe concurrent access
//! - Does NOT cache streaming responses

use async_trait::async_trait;
use moka::future::Cache;
use sha2::{Digest, Sha256};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use crate::{LlmError, LlmProvider, LlmRequest, LlmResponse};

/// Configuration for the LLM cache
#[derive(Debug, Clone)]
pub struct LlmCacheConfig {
    /// Maximum number of cached responses
    pub max_entries: u64,
    /// Time-to-live for cached entries
    pub ttl: Duration,
    /// Time-to-idle (evict if not accessed)
    pub tti: Option<Duration>,
}

impl Default for LlmCacheConfig {
    fn default() -> Self {
        Self {
            max_entries: 1000,
            ttl: Duration::from_secs(3600),       // 1 hour
            tti: Some(Duration::from_secs(1800)), // 30 minutes idle
        }
    }
}

impl LlmCacheConfig {
    /// Aggressive caching for cost savings
    pub fn aggressive() -> Self {
        Self {
            max_entries: 10000,
            ttl: Duration::from_secs(86400), // 24 hours
            tti: None,
        }
    }

    /// Conservative caching for freshness
    pub fn conservative() -> Self {
        Self {
            max_entries: 100,
            ttl: Duration::from_secs(300), // 5 minutes
            tti: Some(Duration::from_secs(60)),
        }
    }
}

/// Calculate cache key from request
fn cache_key(request: &LlmRequest) -> String {
    let mut hasher = Sha256::new();
    hasher.update(request.system.as_bytes());
    hasher.update(b"|");
    hasher.update(request.prompt.as_bytes());
    hasher.update(b"|");
    hasher.update(request.temperature.to_be_bytes());
    hasher.update(b"|");
    hasher.update(request.max_tokens.to_be_bytes());
    hex::encode(hasher.finalize())
}

/// Cached LLM provider wrapper
#[derive(Debug)]
pub struct CachedProvider<P: LlmProvider> {
    inner: P,
    cache: Cache<String, LlmResponse>,
    hits: AtomicU64,
    misses: AtomicU64,
}

impl<P: LlmProvider> CachedProvider<P> {
    /// Create a cached wrapper around an LLM provider
    pub fn new(provider: P, config: LlmCacheConfig) -> Self {
        let mut builder = Cache::builder()
            .max_capacity(config.max_entries)
            .time_to_live(config.ttl);

        if let Some(tti) = config.tti {
            builder = builder.time_to_idle(tti);
        }

        Self {
            inner: provider,
            cache: builder.build(),
            hits: AtomicU64::new(0),
            misses: AtomicU64::new(0),
        }
    }

    /// Create with default configuration
    pub fn wrap(provider: P) -> Self {
        Self::new(provider, LlmCacheConfig::default())
    }

    /// Get cache statistics
    pub fn stats(&self) -> (u64, u64, f64) {
        let hits = self.hits.load(Ordering::Relaxed);
        let misses = self.misses.load(Ordering::Relaxed);
        let total = hits + misses;
        let hit_rate = if total > 0 {
            hits as f64 / total as f64
        } else {
            0.0
        };
        (hits, misses, hit_rate)
    }

    /// Clear the cache
    pub fn clear(&self) {
        self.cache.invalidate_all();
    }

    /// Get current cache size
    pub fn size(&self) -> u64 {
        self.cache.entry_count()
    }
}

#[async_trait]
impl<P: LlmProvider + 'static> LlmProvider for CachedProvider<P> {
    fn name(&self) -> &str {
        "cached"
    }

    async fn is_available(&self) -> bool {
        self.inner.is_available().await
    }

    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        let key = cache_key(&request);

        // Check cache first
        if let Some(cached) = self.cache.get(&key).await {
            self.hits.fetch_add(1, Ordering::Relaxed);
            tracing::debug!(cache_key = %key, "LLM cache hit");
            return Ok(cached);
        }

        // Cache miss - call provider
        self.misses.fetch_add(1, Ordering::Relaxed);
        let response = self.inner.complete(request).await?;

        // Cache the response
        self.cache.insert(key.clone(), response.clone()).await;
        tracing::debug!(cache_key = %key, "LLM cache miss - stored");

        Ok(response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MockProvider;

    #[tokio::test]
    async fn test_cached_provider_caches_responses() {
        let mock = MockProvider::constant("cached response");
        let cached = CachedProvider::wrap(mock);

        // First call - cache miss
        let req = LlmRequest::simple("test prompt");
        let resp1 = cached.complete(req.clone()).await.unwrap();

        // Second call - cache hit
        let resp2 = cached.complete(req).await.unwrap();

        assert_eq!(resp1.content, resp2.content);

        let (hits, misses, _) = cached.stats();
        assert_eq!(hits, 1);
        assert_eq!(misses, 1);
    }

    #[tokio::test]
    async fn test_different_prompts_different_cache_keys() {
        let mock = MockProvider::smart();
        let cached = CachedProvider::wrap(mock);

        let req1 = LlmRequest::simple("prompt 1");
        let req2 = LlmRequest::simple("prompt 2");

        let _ = cached.complete(req1).await.unwrap();
        let _ = cached.complete(req2).await.unwrap();

        let (hits, misses, _) = cached.stats();
        assert_eq!(hits, 0);
        assert_eq!(misses, 2);
    }
}
