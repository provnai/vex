//! Rate limiting for LLM API calls

use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    /// Maximum requests per window
    pub max_requests: u32,
    /// Time window
    pub window: Duration,
    /// Maximum tokens per minute (if applicable)
    pub max_tokens_per_minute: Option<u32>,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 60,  // 60 requests per minute
            window: Duration::from_secs(60),
            max_tokens_per_minute: Some(100_000),
        }
    }
}

impl RateLimitConfig {
    /// Conservative rate limit for free tiers
    pub fn conservative() -> Self {
        Self {
            max_requests: 10,
            window: Duration::from_secs(60),
            max_tokens_per_minute: Some(10_000),
        }
    }

    /// Aggressive rate limit for paid tiers
    pub fn aggressive() -> Self {
        Self {
            max_requests: 500,
            window: Duration::from_secs(60),
            max_tokens_per_minute: Some(1_000_000),
        }
    }
}

/// Request tracking
#[derive(Debug, Clone)]
struct RequestWindow {
    count: u32,
    tokens: u32,
    window_start: Instant,
}

impl Default for RequestWindow {
    fn default() -> Self {
        Self {
            count: 0,
            tokens: 0,
            window_start: Instant::now(),
        }
    }
}

/// Rate limiter for API calls
#[derive(Debug)]
pub struct RateLimiter {
    config: RateLimitConfig,
    /// Per-provider rate tracking
    windows: RwLock<HashMap<String, RequestWindow>>,
}

impl RateLimiter {
    /// Create a new rate limiter
    pub fn new(config: RateLimitConfig) -> Self {
        Self {
            config,
            windows: RwLock::new(HashMap::new()),
        }
    }

    /// Check if a request is allowed (doesn't consume)
    pub async fn check(&self, provider: &str) -> RateLimitResult {
        let windows = self.windows.read().await;
        let window = windows.get(provider).cloned().unwrap_or_default();
        
        self.evaluate(&window)
    }

    /// Acquire a permit (blocks if rate limited)
    pub async fn acquire(&self, provider: &str) -> Result<(), RateLimitError> {
        loop {
            let result = self.try_acquire(provider).await;
            match result {
                Ok(()) => return Ok(()),
                Err(RateLimitError::Limited { retry_after }) => {
                    tokio::time::sleep(retry_after).await;
                }
                Err(e) => return Err(e),
            }
        }
    }

    /// Try to acquire a permit (non-blocking)
    pub async fn try_acquire(&self, provider: &str) -> Result<(), RateLimitError> {
        self.try_acquire_with_tokens(provider, 0).await
    }

    /// Try to acquire a permit with estimated token usage (atomic check)
    /// This prevents race conditions between request counting and token tracking
    pub async fn try_acquire_with_tokens(&self, provider: &str, estimated_tokens: u32) -> Result<(), RateLimitError> {
        let mut windows = self.windows.write().await;
        let window = windows.entry(provider.to_string()).or_default();
        
        // Check if window expired
        let elapsed = window.window_start.elapsed();
        if elapsed >= self.config.window {
            // Reset window
            *window = RequestWindow::default();
        }
        
        // Check request limit
        if window.count >= self.config.max_requests {
            let retry_after = self.config.window - elapsed;
            return Err(RateLimitError::Limited { retry_after });
        }
        
        // Check token limit (if configured and tokens provided)
        if let Some(max_tokens) = self.config.max_tokens_per_minute {
            if estimated_tokens > 0 && window.tokens + estimated_tokens > max_tokens {
                let retry_after = self.config.window - elapsed;
                return Err(RateLimitError::Limited { retry_after });
            }
        }
        
        // Acquire atomically - update both counters under same lock
        window.count += 1;
        window.tokens += estimated_tokens;
        Ok(())
    }

    /// Record additional token usage after completion
    /// Use this to adjust for actual tokens used vs estimated
    pub async fn record_tokens(&self, provider: &str, additional_tokens: u32) {
        let mut windows = self.windows.write().await;
        if let Some(window) = windows.get_mut(provider) {
            window.tokens += additional_tokens;
        }
    }

    /// Get current usage stats
    pub async fn stats(&self, provider: &str) -> RateLimitStats {
        let windows = self.windows.read().await;
        let window = windows.get(provider).cloned().unwrap_or_default();
        
        RateLimitStats {
            requests_used: window.count,
            requests_limit: self.config.max_requests,
            tokens_used: window.tokens,
            tokens_limit: self.config.max_tokens_per_minute,
            window_remaining: self.config.window.saturating_sub(window.window_start.elapsed()),
        }
    }

    fn evaluate(&self, window: &RequestWindow) -> RateLimitResult {
        let elapsed = window.window_start.elapsed();
        if elapsed >= self.config.window {
            return RateLimitResult::Allowed;
        }
        
        if window.count >= self.config.max_requests {
            let retry_after = self.config.window - elapsed;
            return RateLimitResult::Limited { retry_after };
        }
        
        if let Some(max_tokens) = self.config.max_tokens_per_minute {
            if window.tokens >= max_tokens {
                let retry_after = self.config.window - elapsed;
                return RateLimitResult::Limited { retry_after };
            }
        }
        
        RateLimitResult::Allowed
    }
}

/// Rate limit check result
#[derive(Debug, Clone)]
pub enum RateLimitResult {
    Allowed,
    Limited { retry_after: Duration },
}

/// Rate limit error
#[derive(Debug, thiserror::Error)]
pub enum RateLimitError {
    #[error("Rate limited, retry after {retry_after:?}")]
    Limited { retry_after: Duration },
}

/// Rate limit statistics
#[derive(Debug, Clone)]
pub struct RateLimitStats {
    pub requests_used: u32,
    pub requests_limit: u32,
    pub tokens_used: u32,
    pub tokens_limit: Option<u32>,
    pub window_remaining: Duration,
}

/// Rate-limited LLM provider wrapper
pub struct RateLimitedProvider<P> {
    inner: P,
    limiter: Arc<RateLimiter>,
    provider_name: String,
}

impl<P> RateLimitedProvider<P> {
    pub fn new(inner: P, limiter: Arc<RateLimiter>, provider_name: &str) -> Self {
        Self {
            inner,
            limiter,
            provider_name: provider_name.to_string(),
        }
    }

    pub fn inner(&self) -> &P {
        &self.inner
    }

    pub async fn acquire(&self) -> Result<(), RateLimitError> {
        self.limiter.acquire(&self.provider_name).await
    }

    pub async fn stats(&self) -> RateLimitStats {
        self.limiter.stats(&self.provider_name).await
    }
}

/// User tier for rate limiting
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UserTier {
    /// Free tier - limited access
    Free,
    /// Pro tier - increased limits
    Pro,
    /// Enterprise tier - highest limits
    Enterprise,
}

impl UserTier {
    /// Get rate limit config for this tier
    pub fn rate_limit_config(&self) -> RateLimitConfig {
        match self {
            UserTier::Free => RateLimitConfig {
                max_requests: 10,
                window: Duration::from_secs(60),
                max_tokens_per_minute: Some(5_000),
            },
            UserTier::Pro => RateLimitConfig {
                max_requests: 100,
                window: Duration::from_secs(60),
                max_tokens_per_minute: Some(50_000),
            },
            UserTier::Enterprise => RateLimitConfig {
                max_requests: 1000,
                window: Duration::from_secs(60),
                max_tokens_per_minute: Some(500_000),
            },
        }
    }
}

impl Default for UserTier {
    fn default() -> Self {
        UserTier::Free
    }
}

/// Per-user rate limiter with tier-based quotas
#[derive(Debug)]
pub struct UserRateLimiter {
    /// Per-user windows (keyed by user_id)
    user_windows: RwLock<HashMap<String, UserRateLimitState>>,
    /// Default tier for unknown users
    default_tier: UserTier,
    /// User tier overrides
    tier_overrides: RwLock<HashMap<String, UserTier>>,
}

#[derive(Debug, Clone)]
struct UserRateLimitState {
    tier: UserTier,
    window: RequestWindow,
}

impl Default for UserRateLimitState {
    fn default() -> Self {
        Self {
            tier: UserTier::Free,
            window: RequestWindow::default(),
        }
    }
}

impl UserRateLimiter {
    /// Create a new per-user rate limiter
    pub fn new(default_tier: UserTier) -> Self {
        Self {
            user_windows: RwLock::new(HashMap::new()),
            default_tier,
            tier_overrides: RwLock::new(HashMap::new()),
        }
    }

    /// Set a user's tier
    pub async fn set_user_tier(&self, user_id: &str, tier: UserTier) {
        let mut overrides = self.tier_overrides.write().await;
        overrides.insert(user_id.to_string(), tier);
    }

    /// Get a user's current tier
    pub async fn get_user_tier(&self, user_id: &str) -> UserTier {
        let overrides = self.tier_overrides.read().await;
        overrides.get(user_id).copied().unwrap_or(self.default_tier)
    }

    /// Try to acquire a permit for a user
    pub async fn try_acquire(&self, user_id: &str) -> Result<(), RateLimitError> {
        self.try_acquire_with_tokens(user_id, 0).await
    }

    /// Try to acquire a permit with estimated tokens for a user
    pub async fn try_acquire_with_tokens(&self, user_id: &str, estimated_tokens: u32) -> Result<(), RateLimitError> {
        let tier = self.get_user_tier(user_id).await;
        let config = tier.rate_limit_config();
        
        let mut windows = self.user_windows.write().await;
        let state = windows.entry(user_id.to_string()).or_insert_with(|| {
            UserRateLimitState {
                tier,
                window: RequestWindow::default(),
            }
        });
        
        // Update tier if it changed
        state.tier = tier;
        
        // Check if window expired
        let elapsed = state.window.window_start.elapsed();
        if elapsed >= config.window {
            state.window = RequestWindow::default();
        }
        
        // Check request limit
        if state.window.count >= config.max_requests {
            let retry_after = config.window - elapsed;
            return Err(RateLimitError::Limited { retry_after });
        }
        
        // Check token limit
        if let Some(max_tokens) = config.max_tokens_per_minute {
            if estimated_tokens > 0 && state.window.tokens + estimated_tokens > max_tokens {
                let retry_after = config.window - elapsed;
                return Err(RateLimitError::Limited { retry_after });
            }
        }
        
        // Acquire
        state.window.count += 1;
        state.window.tokens += estimated_tokens;
        Ok(())
    }

    /// Get usage stats for a user
    pub async fn user_stats(&self, user_id: &str) -> UserRateLimitStats {
        let tier = self.get_user_tier(user_id).await;
        let config = tier.rate_limit_config();
        
        let windows = self.user_windows.read().await;
        let state = windows.get(user_id).cloned().unwrap_or_default();
        
        let elapsed = state.window.window_start.elapsed();
        let window_remaining = if elapsed >= config.window {
            config.window
        } else {
            config.window - elapsed
        };
        
        UserRateLimitStats {
            user_id: user_id.to_string(),
            tier,
            requests_used: state.window.count,
            requests_limit: config.max_requests,
            tokens_used: state.window.tokens,
            tokens_limit: config.max_tokens_per_minute,
            window_remaining,
        }
    }
}

/// Per-user rate limit statistics
#[derive(Debug, Clone)]
pub struct UserRateLimitStats {
    pub user_id: String,
    pub tier: UserTier,
    pub requests_used: u32,
    pub requests_limit: u32,
    pub tokens_used: u32,
    pub tokens_limit: Option<u32>,
    pub window_remaining: Duration,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter() {
        let config = RateLimitConfig {
            max_requests: 3,
            window: Duration::from_millis(100),
            max_tokens_per_minute: None,
        };
        let limiter = RateLimiter::new(config);

        // Should allow first 3 requests
        assert!(limiter.try_acquire("test").await.is_ok());
        assert!(limiter.try_acquire("test").await.is_ok());
        assert!(limiter.try_acquire("test").await.is_ok());

        // 4th should be limited
        assert!(matches!(
            limiter.try_acquire("test").await,
            Err(RateLimitError::Limited { .. })
        ));

        // Wait for window to reset
        tokio::time::sleep(Duration::from_millis(150)).await;

        // Should allow again
        assert!(limiter.try_acquire("test").await.is_ok());
    }

    #[tokio::test]
    async fn test_stats() {
        let limiter = RateLimiter::new(RateLimitConfig::default());
        
        limiter.try_acquire("provider1").await.unwrap();
        limiter.try_acquire("provider1").await.unwrap();
        
        let stats = limiter.stats("provider1").await;
        assert_eq!(stats.requests_used, 2);
    }

    #[tokio::test]
    async fn test_user_rate_limiter_tiers() {
        let limiter = UserRateLimiter::new(UserTier::Free);
        
        // Default tier should be Free (10 requests/min)
        assert_eq!(limiter.get_user_tier("user1").await, UserTier::Free);
        
        // Set user to Pro tier
        limiter.set_user_tier("user2", UserTier::Pro).await;
        assert_eq!(limiter.get_user_tier("user2").await, UserTier::Pro);
        
        // Free user should be limited after 10 requests
        for _ in 0..10 {
            assert!(limiter.try_acquire("free_user").await.is_ok());
        }
        assert!(matches!(
            limiter.try_acquire("free_user").await,
            Err(RateLimitError::Limited { .. })
        ));
        
        // Pro user should have 100 request limit
        limiter.set_user_tier("pro_user", UserTier::Pro).await;
        for _ in 0..50 {
            assert!(limiter.try_acquire("pro_user").await.is_ok());
        }
        let stats = limiter.user_stats("pro_user").await;
        assert_eq!(stats.tier, UserTier::Pro);
        assert_eq!(stats.requests_used, 50);
        assert_eq!(stats.requests_limit, 100);
    }
}
