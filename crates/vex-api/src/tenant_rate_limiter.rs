//! Tenant-scoped rate limiting using governor (GCRA algorithm)
//!
//! Provides per-tenant rate limiting for API endpoints using the GCRA
//! (Generic Cell Rate Algorithm) which is efficient and doesn't require
//! background maintenance threads.
//!
//! # 2025 Best Practices
//! - Uses governor crate for efficient GCRA implementation
//! - Per-tenant keyed rate limiting (by header or API key)
//! - Configurable quotas per tier

use governor::{
    clock::{Clock, DefaultClock},
    state::{InMemoryState, NotKeyed},
    Quota, RateLimiter as Governor,
};
use std::collections::HashMap;
use std::num::NonZeroU32;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

/// Rate limit tier for different tenant types
#[derive(
    Debug,
    Clone,
    Copy,
    PartialEq,
    Eq,
    Hash,
    serde::Serialize,
    serde::Deserialize,
    utoipa::ToSchema,
    Default,
)]
pub enum RateLimitTier {
    /// Free tier: 10 requests/minute
    #[default]
    Free,
    /// Standard tier: 100 requests/minute
    Standard,
    /// Pro tier: 1000 requests/minute
    Pro,
    /// Unlimited (for internal services)
    Unlimited,
}

impl RateLimitTier {
    /// Get the quota for this tier
    pub fn quota(&self) -> Option<Quota> {
        match self {
            Self::Free => Some(Quota::per_minute(NonZeroU32::new(10).unwrap())),
            Self::Standard => Some(Quota::per_minute(NonZeroU32::new(100).unwrap())),
            Self::Pro => Some(Quota::per_minute(NonZeroU32::new(1000).unwrap())),
            Self::Unlimited => None, // No limiting
        }
    }
}

/// Per-tenant rate limiter state
type TenantLimiter = Governor<NotKeyed, InMemoryState, DefaultClock>;

/// Tenant-scoped rate limiter
#[derive(Debug)]
pub struct TenantRateLimiter {
    /// Per-tenant limiters
    limiters: RwLock<HashMap<String, Arc<TenantLimiter>>>,
    /// Default tier for new tenants
    default_tier: RateLimitTier,
    /// Tier assignments per tenant
    tier_assignments: RwLock<HashMap<String, RateLimitTier>>,
}

impl Default for TenantRateLimiter {
    fn default() -> Self {
        Self::new(RateLimitTier::Free)
    }
}

impl TenantRateLimiter {
    /// Create a new tenant rate limiter with a default tier
    pub fn new(default_tier: RateLimitTier) -> Self {
        Self {
            limiters: RwLock::new(HashMap::new()),
            default_tier,
            tier_assignments: RwLock::new(HashMap::new()),
        }
    }

    /// Assign a tier to a tenant
    pub async fn set_tier(&self, tenant_id: &str, tier: RateLimitTier) {
        let mut assignments = self.tier_assignments.write().await;
        assignments.insert(tenant_id.to_string(), tier);

        // Remove cached limiter so it gets recreated with new tier
        let mut limiters = self.limiters.write().await;
        limiters.remove(tenant_id);
    }

    /// Get a tenant's tier
    pub async fn get_tier(&self, tenant_id: &str) -> RateLimitTier {
        let assignments = self.tier_assignments.read().await;
        assignments
            .get(tenant_id)
            .copied()
            .unwrap_or(self.default_tier)
    }

    /// Check if a request is allowed for a tenant
    pub async fn check(&self, tenant_id: &str) -> Result<(), Duration> {
        let tier = self.get_tier(tenant_id).await;

        // Unlimited tier always passes
        let quota = match tier.quota() {
            Some(q) => q,
            None => return Ok(()),
        };

        // Get or create limiter for this tenant
        let limiter = self.get_or_create_limiter(tenant_id, quota).await;

        match limiter.check() {
            Ok(_) => Ok(()),
            Err(not_until) => {
                let wait = not_until.wait_time_from(DefaultClock::default().now());
                Err(wait)
            }
        }
    }

    /// Get or create a limiter for a tenant
    async fn get_or_create_limiter(&self, tenant_id: &str, quota: Quota) -> Arc<TenantLimiter> {
        // Fast path: check if exists
        {
            let limiters = self.limiters.read().await;
            if let Some(limiter) = limiters.get(tenant_id) {
                return limiter.clone();
            }
        }

        // Slow path: create new limiter
        let mut limiters = self.limiters.write().await;

        // Double-check after acquiring write lock
        if let Some(limiter) = limiters.get(tenant_id) {
            return limiter.clone();
        }

        let limiter = Arc::new(Governor::direct(quota));
        limiters.insert(tenant_id.to_string(), limiter.clone());
        limiter
    }

    /// Cleanup stale limiters (call periodically)
    pub async fn cleanup(&self) {
        let limiters = self.limiters.write().await;
        // In a production system, you'd track last activity and remove inactive ones
        // For now, just log the count
        tracing::debug!(limiter_count = limiters.len(), "Tenant limiter cleanup");

        // Could add logic here to remove limiters unused for > X minutes
        // but governor's memory footprint is tiny, so not critical
        let _ = limiters; // Suppress unused warning
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rate_limiter_allows_within_quota() {
        let limiter = TenantRateLimiter::new(RateLimitTier::Standard);

        // Should allow requests within quota
        for _ in 0..10 {
            assert!(limiter.check("tenant1").await.is_ok());
        }
    }

    #[tokio::test]
    async fn test_rate_limiter_blocks_over_quota() {
        let limiter = TenantRateLimiter::new(RateLimitTier::Free);

        // Exhaust the quota (10 requests for Free tier)
        for _ in 0..10 {
            let _ = limiter.check("tenant1").await;
        }

        // Next request should be rate limited
        let result = limiter.check("tenant1").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_different_tenants_independent() {
        let limiter = TenantRateLimiter::new(RateLimitTier::Free);

        // Exhaust tenant1's quota
        for _ in 0..15 {
            let _ = limiter.check("tenant1").await;
        }

        // tenant2 should still work
        assert!(limiter.check("tenant2").await.is_ok());
    }

    #[tokio::test]
    async fn test_unlimited_tier() {
        let limiter = TenantRateLimiter::new(RateLimitTier::Free);
        limiter.set_tier("vip", RateLimitTier::Unlimited).await;

        // Should never be rate limited
        for _ in 0..1000 {
            assert!(limiter.check("vip").await.is_ok());
        }
    }
}
