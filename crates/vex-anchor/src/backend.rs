//! Core backend trait for anchoring

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use vex_core::Hash;

use crate::error::AnchorError;

/// Metadata about the data being anchored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorMetadata {
    /// Tenant identifier
    pub tenant_id: String,
    /// Number of events in this anchor batch
    pub event_count: u64,
    /// Timestamp when anchor was requested
    pub timestamp: DateTime<Utc>,
    /// Optional description
    pub description: Option<String>,
}

impl AnchorMetadata {
    /// Create new anchor metadata
    pub fn new(tenant_id: impl Into<String>, event_count: u64) -> Self {
        Self {
            tenant_id: tenant_id.into(),
            event_count,
            timestamp: Utc::now(),
            description: None,
        }
    }

    /// Add a description
    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = Some(desc.into());
        self
    }
}

/// Receipt proving that a root was anchored
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnchorReceipt {
    /// Backend that performed the anchoring
    pub backend: String,
    /// The Merkle root that was anchored
    pub root_hash: String,
    /// Unique identifier from the backend (commit hash, tx hash, etc.)
    pub anchor_id: String,
    /// When the anchoring occurred
    pub anchored_at: DateTime<Utc>,
    /// Optional proof data (OTS proof, blob commitment, etc.)
    pub proof: Option<String>,
    /// Metadata that was anchored
    pub metadata: AnchorMetadata,
}

impl AnchorReceipt {
    /// Export receipt as JSON
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Import receipt from JSON
    pub fn from_json(json: &str) -> Result<Self, serde_json::Error> {
        serde_json::from_str(json)
    }
}

/// Trait for anchoring backends
///
/// Implementations should be:
/// - Append-only (no deletion of anchors)
/// - Tamper-evident (modifications are detectable)
/// - Verifiable (anchors can be independently verified)
#[async_trait]
pub trait AnchorBackend: Send + Sync {
    /// Anchor a Merkle root to the external system
    ///
    /// Returns a receipt that can be used to verify the anchor later.
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError>;

    /// Verify that a previously issued receipt is still valid
    ///
    /// Returns `true` if the anchor exists and matches the receipt.
    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError>;

    /// Get the human-readable name of this backend
    fn name(&self) -> &str;

    /// Check if the backend is available and healthy
    async fn is_healthy(&self) -> bool;
}
