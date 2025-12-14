//! Time horizon definitions for agents

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

/// Time horizon scale for an agent
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TimeHorizon {
    /// Immediate: 0-5 minutes of context
    Immediate,
    /// Short-term: 5-60 minutes
    ShortTerm,
    /// Medium-term: 1-24 hours
    MediumTerm,
    /// Long-term: days to weeks
    LongTerm,
    /// Permanent: never expires
    Permanent,
}

impl TimeHorizon {
    /// Get the duration for this horizon
    pub fn duration(&self) -> Option<Duration> {
        match self {
            Self::Immediate => Some(Duration::minutes(5)),
            Self::ShortTerm => Some(Duration::hours(1)),
            Self::MediumTerm => Some(Duration::hours(24)),
            Self::LongTerm => Some(Duration::weeks(1)),
            Self::Permanent => None,
        }
    }

    /// Get compression level for this horizon
    pub fn compression_level(&self) -> f64 {
        match self {
            Self::Immediate => 0.0,  // No compression
            Self::ShortTerm => 0.2,  // Light
            Self::MediumTerm => 0.5, // Moderate
            Self::LongTerm => 0.7,   // Heavy
            Self::Permanent => 0.9,  // Maximum
        }
    }

    /// Check if a timestamp is within this horizon
    pub fn contains(&self, timestamp: DateTime<Utc>) -> bool {
        match self.duration() {
            Some(d) => Utc::now() - timestamp <= d,
            None => true,
        }
    }

    /// Get recommended horizon for agent depth
    pub fn for_depth(depth: u8) -> Self {
        match depth {
            0 => Self::LongTerm,   // Root agents have long memory
            1 => Self::MediumTerm, // First-level children
            2 => Self::ShortTerm,  // Second-level
            _ => Self::Immediate,  // Deeper agents are ephemeral
        }
    }
}

/// Configuration for time horizon behavior
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HorizonConfig {
    /// The active horizon
    pub horizon: TimeHorizon,
    /// Maximum entries to keep
    pub max_entries: usize,
    /// Whether to auto-compress old entries
    pub auto_compress: bool,
    /// Whether to auto-evict expired entries
    pub auto_evict: bool,
}

impl Default for HorizonConfig {
    fn default() -> Self {
        Self {
            horizon: TimeHorizon::MediumTerm,
            max_entries: 100,
            auto_compress: true,
            auto_evict: true,
        }
    }
}

impl HorizonConfig {
    /// Create config for a given agent depth
    pub fn for_depth(depth: u8) -> Self {
        let horizon = TimeHorizon::for_depth(depth);
        let max_entries = match horizon {
            TimeHorizon::Immediate => 10,
            TimeHorizon::ShortTerm => 25,
            TimeHorizon::MediumTerm => 50,
            TimeHorizon::LongTerm => 100,
            TimeHorizon::Permanent => 500,
        };
        Self {
            horizon,
            max_entries,
            auto_compress: true,
            auto_evict: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_horizon_duration() {
        assert!(TimeHorizon::Immediate.duration().is_some());
        assert!(TimeHorizon::Permanent.duration().is_none());
    }

    #[test]
    fn test_horizon_for_depth() {
        assert_eq!(TimeHorizon::for_depth(0), TimeHorizon::LongTerm);
        assert_eq!(TimeHorizon::for_depth(3), TimeHorizon::Immediate);
    }

    #[test]
    fn test_contains() {
        let now = Utc::now();
        assert!(TimeHorizon::Immediate.contains(now));
    }
}
