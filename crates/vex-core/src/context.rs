//! Context packets for VEX agents
//!
//! A [`ContextPacket`] is the unit of information passed between agents,
//! with temporal metadata and cryptographic hashing.

use crate::merkle::Hash;
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use uuid::Uuid;

/// Compression level for context packets
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// Full fidelity - no compression
    Full,
    /// Summary - moderate compression
    Summary,
    /// Abstract - high compression, key points only
    Abstract,
    /// Minimal - extreme compression
    Minimal,
}

impl CompressionLevel {
    /// Get the numeric compression ratio (0.0 = full, 1.0 = minimal)
    pub fn ratio(&self) -> f64 {
        match self {
            Self::Full => 0.0,
            Self::Summary => 0.3,
            Self::Abstract => 0.6,
            Self::Minimal => 0.9,
        }
    }
}

/// A context packet - the unit of information in VEX
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextPacket {
    /// Unique identifier for this packet
    pub id: Uuid,
    /// The actual content
    pub content: String,
    /// When this context was created
    pub created_at: DateTime<Utc>,
    /// When this context expires (if applicable)
    pub expires_at: Option<DateTime<Utc>>,
    /// Compression level applied
    pub compression: CompressionLevel,
    /// SHA-256 hash of the content
    pub hash: Hash,
    /// Hash of the parent packet (for chaining)
    pub parent_hash: Option<Hash>,
    /// Source agent ID
    pub source_agent: Option<Uuid>,
    /// Importance score (0.0 - 1.0)
    pub importance: f64,
}

impl ContextPacket {
    /// Create a new context packet with the given content
    pub fn new(content: &str) -> Self {
        let hash = Self::compute_hash(content);
        Self {
            id: Uuid::new_v4(),
            content: content.to_string(),
            created_at: Utc::now(),
            expires_at: None,
            compression: CompressionLevel::Full,
            hash,
            parent_hash: None,
            source_agent: None,
            importance: 0.5,
        }
    }

    /// Create a context packet with a TTL (time-to-live)
    pub fn with_ttl(content: &str, ttl: Duration) -> Self {
        let mut packet = Self::new(content);
        packet.expires_at = Some(Utc::now() + ttl);
        packet
    }

    /// Compute SHA-256 hash of content
    pub fn compute_hash(content: &str) -> Hash {
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Hash(hasher.finalize().into())
    }

    /// Check if this packet has expired
    pub fn is_expired(&self) -> bool {
        self.expires_at.map_or(false, |exp| Utc::now() > exp)
    }

    /// Get the age of this packet
    pub fn age(&self) -> Duration {
        Utc::now().signed_duration_since(self.created_at)
    }

    /// Create a compressed version of this packet
    pub fn compress(&self, level: CompressionLevel) -> Self {
        // In a real implementation, this would use an LLM to summarize
        // For now, we just truncate based on compression ratio
        let max_len = ((1.0 - level.ratio()) * self.content.len() as f64) as usize;
        let compressed_content = if max_len < self.content.len() {
            format!("{}...", &self.content[..max_len.max(10)])
        } else {
            self.content.clone()
        };

        let mut packet = Self::new(&compressed_content);
        packet.compression = level;
        packet.parent_hash = Some(self.hash.clone());
        packet.source_agent = self.source_agent;
        packet.importance = self.importance;
        packet
    }

    /// Chain this packet to a parent
    pub fn chain_to(&mut self, parent: &ContextPacket) {
        self.parent_hash = Some(parent.hash.clone());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_packet() {
        let packet = ContextPacket::new("Hello, world!");
        assert_eq!(packet.content, "Hello, world!");
        assert_eq!(packet.compression, CompressionLevel::Full);
        assert!(!packet.is_expired());
    }

    #[test]
    fn test_packet_with_ttl() {
        let packet = ContextPacket::with_ttl("Temporary data", Duration::hours(1));
        assert!(packet.expires_at.is_some());
        assert!(!packet.is_expired());
    }

    #[test]
    fn test_compress_packet() {
        let packet = ContextPacket::new("This is a long piece of content that should be compressed when needed.");
        let compressed = packet.compress(CompressionLevel::Summary);
        assert_eq!(compressed.compression, CompressionLevel::Summary);
        assert!(compressed.content.len() <= packet.content.len());
    }
}
