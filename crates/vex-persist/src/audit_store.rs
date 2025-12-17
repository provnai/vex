//! Audit log storage with Merkle verification

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::backend::{StorageBackend, StorageError, StorageExt};
use vex_core::{Hash, MerkleTree};

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuditEventType {
    AgentCreated,
    AgentExecuted,
    DebateStarted,
    DebateRound,
    DebateConcluded,
    ConsensusReached,
    ContextStored,
    PaymentInitiated,
    PaymentCompleted,
    Custom(String),
}

/// Single audit event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    /// Unique event ID
    pub id: Uuid,
    /// Event type
    pub event_type: AuditEventType,
    /// Timestamp
    pub timestamp: DateTime<Utc>,
    /// Agent involved (if any)
    pub agent_id: Option<Uuid>,
    /// Event data (JSON)
    pub data: serde_json::Value,
    /// Hash of this event
    pub hash: Hash,
    /// Hash of previous event (chain)
    pub previous_hash: Option<Hash>,
    /// Monotonic sequence number for ordering verification
    pub sequence_number: u64,
}

impl AuditEvent {
    /// Fields that should be redacted from audit log data for security
    const SENSITIVE_FIELDS: &'static [&'static str] = &[
        "password", "secret", "token", "api_key", "apikey", "key",
        "authorization", "auth", "credential", "private_key", "privatekey"
    ];

    /// Create a new audit event with sanitized data
    /// Note: sequence_number should be provided by the AuditStore for proper ordering
    pub fn new(
        event_type: AuditEventType,
        agent_id: Option<Uuid>,
        data: serde_json::Value,
        sequence_number: u64,
    ) -> Self {
        let id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Sanitize sensitive fields from data
        let data = Self::sanitize_data(data);

        // Compute hash including sequence number for tamper detection
        let content = format!("{:?}:{}:{}:{:?}", event_type, timestamp.timestamp(), sequence_number, data);
        let hash = Hash::digest(content.as_bytes());

        Self {
            id,
            event_type,
            timestamp,
            agent_id,
            data,
            hash,
            previous_hash: None,
            sequence_number,
        }
    }

    /// Sanitize sensitive fields from audit data
    fn sanitize_data(value: serde_json::Value) -> serde_json::Value {
        match value {
            serde_json::Value::Object(mut map) => {
                for key in map.keys().cloned().collect::<Vec<_>>() {
                    let lower_key = key.to_lowercase();
                    if Self::SENSITIVE_FIELDS.iter().any(|f| lower_key.contains(f)) {
                        map.insert(key, serde_json::Value::String("[REDACTED]".to_string()));
                    } else if let Some(v) = map.remove(&key) {
                        map.insert(key, Self::sanitize_data(v));
                    }
                }
                serde_json::Value::Object(map)
            }
            serde_json::Value::Array(arr) => {
                serde_json::Value::Array(arr.into_iter().map(Self::sanitize_data).collect())
            }
            other => other,
        }
    }

    /// Create with chained previous hash
    pub fn chained(
        event_type: AuditEventType,
        agent_id: Option<Uuid>,
        data: serde_json::Value,
        previous_hash: Hash,
        sequence_number: u64,
    ) -> Self {
        let mut event = Self::new(event_type, agent_id, data, sequence_number);
        event.previous_hash = Some(previous_hash.clone());
        // Rehash including previous hash and sequence
        let content = format!("{}:{}:{}", event.hash, previous_hash, sequence_number);
        event.hash = Hash::digest(content.as_bytes());
        event
    }
}

/// Audit store for compliance logging
#[derive(Debug)]
pub struct AuditStore<B: StorageBackend + ?Sized> {
    backend: Arc<B>,
    prefix: String,
    /// Last event hash (for chaining)
    last_hash: tokio::sync::RwLock<Option<Hash>>,
    /// Monotonic sequence counter for ordering verification
    sequence_counter: std::sync::atomic::AtomicU64,
}

impl<B: StorageBackend + ?Sized> AuditStore<B> {
    /// Create a new audit store
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            prefix: "audit:".to_string(),
            last_hash: tokio::sync::RwLock::new(None),
            sequence_counter: std::sync::atomic::AtomicU64::new(0),
        }
    }

    fn event_key(&self, id: Uuid) -> String {
        format!("{}event:{}", self.prefix, id)
    }

    fn chain_key(&self) -> String {
        format!("{}chain", self.prefix)
    }

    /// Get next sequence number atomically
    fn next_sequence(&self) -> u64 {
        self.sequence_counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Log an audit event (automatically chained with sequence number)
    pub async fn log(
        &self,
        event_type: AuditEventType,
        agent_id: Option<Uuid>,
        data: serde_json::Value,
    ) -> Result<AuditEvent, StorageError> {
        let mut last_hash = self.last_hash.write().await;
        let seq = self.next_sequence();

        let event = match &*last_hash {
            Some(prev) => AuditEvent::chained(event_type, agent_id, data, prev.clone(), seq),
            None => AuditEvent::new(event_type, agent_id, data, seq),
        };

        // Store event
        self.backend.set(&self.event_key(event.id), &event).await?;

        // Update chain
        let mut chain: Vec<Uuid> = self
            .backend
            .get(&self.chain_key())
            .await?
            .unwrap_or_default();
        chain.push(event.id);
        self.backend.set(&self.chain_key(), &chain).await?;

        // Update last hash
        *last_hash = Some(event.hash.clone());

        Ok(event)
    }

    /// Get event by ID
    pub async fn get(&self, id: Uuid) -> Result<Option<AuditEvent>, StorageError> {
        self.backend.get(&self.event_key(id)).await
    }

    /// Get all events in chain order
    pub async fn get_chain(&self) -> Result<Vec<AuditEvent>, StorageError> {
        let chain: Vec<Uuid> = self
            .backend
            .get(&self.chain_key())
            .await?
            .unwrap_or_default();

        let mut events = Vec::new();
        for id in chain {
            if let Some(event) = self.get(id).await? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Build Merkle tree of all events
    pub async fn build_merkle_tree(&self) -> Result<MerkleTree, StorageError> {
        let events = self.get_chain().await?;
        let leaves: Vec<(String, Hash)> = events
            .iter()
            .map(|e| (e.id.to_string(), e.hash.clone()))
            .collect();
        Ok(MerkleTree::from_leaves(leaves))
    }

    /// Verify chain integrity
    pub async fn verify_chain(&self) -> Result<bool, StorageError> {
        let events = self.get_chain().await?;

        for (i, event) in events.iter().enumerate() {
            if i == 0 {
                // First event should have no previous hash
                if event.previous_hash.is_some() {
                    tracing::warn!("Chain integrity failed: first event has previous_hash");
                    return Ok(false);
                }
            } else {
                // Check chain link - verify prev_hash matches previous event's hash
                match (&event.previous_hash, events.get(i - 1)) {
                    (Some(prev_hash), Some(prev_event)) => {
                        // Verify that this event's previous_hash references the previous event
                        // The chained() constructor combines (event_hash, previous_hash) to create new hash
                        // So we verify the link by checking if prev_hash was derived from prev_event
                        let expected = &prev_event.hash;

                        // For a proper chain, prev_hash should match prev_event's hash
                        // (or be derived from it - depends on chained() implementation)
                        if prev_hash != expected {
                            tracing::warn!(
                                "Chain integrity failed at event {}: expected prev_hash {:?}, got {:?}",
                                event.id, expected.to_hex(), prev_hash.to_hex()
                            );
                            return Ok(false);
                        }
                    }
                    (None, _) => {
                        tracing::warn!(
                            "Chain integrity failed: event {} has no previous_hash",
                            event.id
                        );
                        return Ok(false);
                    }
                    (_, None) => {
                        tracing::warn!(
                            "Chain integrity failed: previous event not found for {}",
                            event.id
                        );
                        return Ok(false);
                    }
                }
            }
        }

        tracing::info!("Chain integrity verified: {} events", events.len());
        Ok(true)
    }

    /// Export audit trail for compliance
    pub async fn export(&self) -> Result<AuditExport, StorageError> {
        let events = self.get_chain().await?;
        let merkle_tree = self.build_merkle_tree().await?;

        Ok(AuditExport {
            events,
            merkle_root: merkle_tree.root_hash().map(|h| h.to_string()),
            exported_at: Utc::now(),
            verified: self.verify_chain().await.unwrap_or(false),
        })
    }
}

/// Audit export for compliance reporting
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditExport {
    pub events: Vec<AuditEvent>,
    pub merkle_root: Option<String>,
    pub exported_at: DateTime<Utc>,
    pub verified: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[tokio::test]
    async fn test_audit_store() {
        let backend = Arc::new(MemoryBackend::new());
        let store = AuditStore::new(backend);

        // Log events
        let _e1 = store
            .log(
                AuditEventType::AgentCreated,
                Some(Uuid::new_v4()),
                serde_json::json!({"name": "TestAgent"}),
            )
            .await
            .unwrap();

        let e2 = store
            .log(
                AuditEventType::AgentExecuted,
                Some(Uuid::new_v4()),
                serde_json::json!({"prompt": "test"}),
            )
            .await
            .unwrap();

        // Verify chain
        assert!(e2.previous_hash.is_some());

        // Get chain
        let chain = store.get_chain().await.unwrap();
        assert_eq!(chain.len(), 2);

        // Build Merkle tree
        let tree = store.build_merkle_tree().await.unwrap();
        assert!(tree.root_hash().is_some());

        // Export
        let export = store.export().await.unwrap();
        assert_eq!(export.events.len(), 2);
        assert!(export.merkle_root.is_some());
    }
}
