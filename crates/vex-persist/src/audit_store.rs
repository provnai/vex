//! Audit log storage with Merkle verification

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

use crate::backend::{StorageBackend, StorageError, StorageExt};
use vex_core::{Hash, MerkleTree};

use vex_core::audit::{ActorType, AuditEvent, AuditEventType, HashParams};

/// Per-tenant chain state for proper multi-tenancy isolation
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ChainState {
    /// Last event hash for this tenant
    last_hash: Option<Hash>,
    /// Monotonic sequence counter for this tenant
    sequence: u64,
}

/// Audit store for compliance logging
///
/// # Multi-Tenancy
/// Chain state (hash and sequence) is now stored per-tenant in the backend,
/// ensuring tenant isolation and preventing cross-tenant chain corruption.
#[derive(Debug)]
pub struct AuditStore<B: StorageBackend + ?Sized> {
    backend: Arc<B>,
    prefix: String,
}

impl<B: StorageBackend + ?Sized> AuditStore<B> {
    /// Create a new audit store
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            prefix: "audit:".to_string(),
        }
    }

    fn event_key(&self, tenant_id: &str, id: Uuid) -> String {
        format!("{}tenant:{}:event:{}", self.prefix, tenant_id, id)
    }

    fn chain_key(&self, tenant_id: &str) -> String {
        format!("{}tenant:{}:chain", self.prefix, tenant_id)
    }

    fn chain_state_key(&self, tenant_id: &str) -> String {
        format!("{}tenant:{}:chain_state", self.prefix, tenant_id)
    }

    /// Get per-tenant chain state from storage
    async fn get_chain_state(&self, tenant_id: &str) -> Result<ChainState, StorageError> {
        self.backend
            .get(&self.chain_state_key(tenant_id))
            .await
            .map(|opt| opt.unwrap_or_default())
    }

    /// Update per-tenant chain state in storage
    async fn set_chain_state(
        &self,
        tenant_id: &str,
        state: &ChainState,
    ) -> Result<(), StorageError> {
        self.backend
            .set(&self.chain_state_key(tenant_id), state)
            .await
    }

    /// Log an audit event (automatically chained with sequence number)
    ///
    /// Chain state is stored per-tenant to ensure proper isolation.
    pub async fn log(
        &self,
        tenant_id: &str,
        event_type: AuditEventType,
        actor: ActorType,
        agent_id: Option<Uuid>,
        data: serde_json::Value,
    ) -> Result<AuditEvent, StorageError> {
        // Pseudonymize actor to protect PII (Centralized in vex-core)
        let actor = actor.pseudonymize();

        // Get per-tenant chain state
        let mut chain_state = self.get_chain_state(tenant_id).await?;
        let seq = chain_state.sequence;

        let mut event = match &chain_state.last_hash {
            Some(prev) => AuditEvent::chained(event_type, agent_id, data, prev.clone(), seq),
            None => AuditEvent::new(event_type, agent_id, data, seq),
        };

        // Set actor after creation to override default system actor
        event.actor = actor;

        // Rehash after setting actor
        event.hash = AuditEvent::compute_hash(HashParams {
            event_type: &event.event_type,
            timestamp: event.timestamp,
            sequence_number: event.sequence_number,
            data: &event.data,
            actor: &event.actor,
            rationale: &event.rationale,
            policy_version: &event.policy_version,
            data_provenance_hash: &event.data_provenance_hash,
            human_review_required: event.human_review_required,
            approval_count: event.approval_signatures.len(),
        });

        if let Some(prev) = &event.previous_hash {
            event.hash = AuditEvent::compute_chained_hash(&event.hash, prev, event.sequence_number);
        }

        // Store event
        self.backend
            .set(&self.event_key(tenant_id, event.id), &event)
            .await?;

        // Update chain index
        let mut chain: Vec<Uuid> = self
            .backend
            .get(&self.chain_key(tenant_id))
            .await?
            .unwrap_or_default();
        chain.push(event.id);
        self.backend.set(&self.chain_key(tenant_id), &chain).await?;

        // Update per-tenant chain state
        chain_state.last_hash = Some(event.hash.clone());
        chain_state.sequence += 1;
        self.set_chain_state(tenant_id, &chain_state).await?;

        Ok(event)
    }

    /// Get event by ID
    pub async fn get(&self, tenant_id: &str, id: Uuid) -> Result<Option<AuditEvent>, StorageError> {
        self.backend.get(&self.event_key(tenant_id, id)).await
    }

    /// Get all events in chain order
    pub async fn get_chain(&self, tenant_id: &str) -> Result<Vec<AuditEvent>, StorageError> {
        let chain: Vec<Uuid> = self
            .backend
            .get(&self.chain_key(tenant_id))
            .await?
            .unwrap_or_default();

        let mut events = Vec::new();
        for id in chain {
            if let Some(event) = self.get(tenant_id, id).await? {
                events.push(event);
            }
        }
        Ok(events)
    }

    /// Build Merkle tree of all events for a tenant
    pub async fn build_merkle_tree(&self, tenant_id: &str) -> Result<MerkleTree, StorageError> {
        let events = self.get_chain(tenant_id).await?;
        let leaves: Vec<(String, Hash)> = events
            .iter()
            .map(|e| (e.id.to_string(), e.hash.clone()))
            .collect();
        Ok(MerkleTree::from_leaves(leaves))
    }

    /// Verify chain integrity for a tenant
    pub async fn verify_chain(&self, tenant_id: &str) -> Result<bool, StorageError> {
        let events = self.get_chain(tenant_id).await?;

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
                        let expected = &prev_event.hash;

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

        tracing::info!(
            "Chain integrity verified for tenant {}: {} events",
            tenant_id,
            events.len()
        );
        Ok(true)
    }

    /// Export audit trail for compliance for a tenant
    pub async fn export(&self, tenant_id: &str) -> Result<AuditExport, StorageError> {
        let events = self.get_chain(tenant_id).await?;
        let merkle_tree = self.build_merkle_tree(tenant_id).await?;

        Ok(AuditExport {
            events,
            merkle_root: merkle_tree.root_hash().map(|h| h.to_string()),
            exported_at: Utc::now(),
            verified: self.verify_chain(tenant_id).await.unwrap_or(false),
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

impl AuditExport {
    /// Export to OCSF v1.7.0 format (Open Cybersecurity Schema Framework)
    /// Uses Detection Finding class (class_uid: 2004) for AI agent events
    /// See: https://schema.ocsf.io/1.7.0/classes/detection_finding
    pub fn to_ocsf(&self) -> Vec<serde_json::Value> {
        self.events
            .iter()
            .map(|event| {
                serde_json::json!({
                    // OCSF Base Event attributes
                    "class_uid": 2004, // Detection Finding
                    "class_name": "Detection Finding",
                    "category_uid": 2, // Findings
                    "category_name": "Findings",
                    "severity_id": 1, // Informational
                    "activity_id": 1, // Create
                    "activity_name": "Create",
                    "status_id": 1, // Success
                    "time": event.timestamp.timestamp(),
                    "timezone_offset": 0,

                    // Finding-specific attributes
                    "finding_info": {
                        "uid": event.id.to_string(),
                        "title": format!("{:?}", event.event_type),
                        "desc": event.rationale.clone().unwrap_or_default(),
                        "created_time": event.timestamp.timestamp(),
                    },

                    // Actor information (ISO 42001 A.6.2.8)
                    "actor": {
                        "type_uid": match &event.actor {
                            ActorType::Bot(_) => 2,
                            ActorType::Human(_) => 1,
                            ActorType::System => 0,
                        },
                        "type": match &event.actor {
                            ActorType::Bot(id) => format!("Bot:{}", id),
                            ActorType::Human(name) => format!("Human:{}", name),
                            ActorType::System => "System".to_string(),
                        },
                    },

                    // VEX-specific extensions
                    "unmapped": {
                        "vex_event_type": format!("{:?}", event.event_type),
                        "vex_hash": event.hash.to_hex(),
                        "vex_sequence": event.sequence_number,
                        "vex_policy_version": event.policy_version.clone(),
                        "vex_data_provenance": event.data_provenance_hash.as_ref().map(|h| h.to_hex()),
                        "vex_human_review_required": event.human_review_required,
                        "vex_merkle_root": self.merkle_root.clone(),
                    },

                    // Metadata
                    "metadata": {
                        "version": "1.7.0",
                        "product": {
                            "name": "VEX Protocol",
                            "vendor_name": "ProvnAI",
                            "version": env!("CARGO_PKG_VERSION"),
                        },
                    },
                })
            })
            .collect()
    }

    /// Export to Splunk HEC format (HTTP Event Collector)
    /// Uses epoch timestamps and proper metadata placement
    /// See: https://docs.splunk.com/Documentation/Splunk/latest/Data/FormateventsforHTTPEventCollector
    pub fn to_splunk_hec(&self, index: &str, source: &str) -> Vec<serde_json::Value> {
        self.events
            .iter()
            .map(|event| {
                serde_json::json!({
                    // Splunk metadata (top-level)
                    "time": event.timestamp.timestamp_millis() as f64 / 1000.0,
                    "host": "vex-protocol",
                    "source": source,
                    "sourcetype": "vex:audit:json",
                    "index": index,

                    // Event data (sanitized for external export - HIGH-2 fix)
                    "event": {
                        "id": event.id.to_string(),
                        "type": format!("{:?}", event.event_type),
                        "timestamp": event.timestamp.to_rfc3339(),
                        "agent_id": event.agent_id.map(|id| id.to_string()),
                        "data": AuditEvent::sanitize_data(event.data.clone()),
                        "hash": event.hash.to_hex(),
                        "sequence": event.sequence_number,
                        // ISO 42001 fields
                        "actor": match &event.actor {
                            ActorType::Bot(id) => serde_json::json!({"type": "bot", "id": id.to_string()}),
                            ActorType::Human(name) => serde_json::json!({"type": "human", "name": name}),
                            ActorType::System => serde_json::json!({"type": "system"}),
                        },
                        "rationale": event.rationale.clone(),
                        "policy_version": event.policy_version.clone(),
                        "human_review_required": event.human_review_required,
                    },

                    // Indexed fields (for fast searching)
                    "fields": {
                        "event_type": format!("{:?}", event.event_type),
                        "merkle_root": self.merkle_root.clone(),
                        "verified": self.verified,
                    },
                })
            })
            .collect()
    }

    /// Export to Datadog logs format
    /// Uses reserved attributes for proper log correlation
    /// See: https://docs.datadoghq.com/logs/log_configuration/attributes_naming_convention
    pub fn to_datadog(&self, service: &str, env: &str) -> Vec<serde_json::Value> {
        self.events
            .iter()
            .map(|event| {
                serde_json::json!({
                    // Datadog reserved attributes
                    "ddsource": "vex-protocol",
                    "ddtags": format!("env:{},service:{}", env, service),
                    "hostname": "vex-audit",
                    "service": service,
                    "status": "info",

                    // Timestamp in ISO8601
                    "timestamp": event.timestamp.to_rfc3339(),

                    // Message for log stream
                    "message": format!(
                        "[{}] {} - seq:{} hash:{}",
                        format!("{:?}", event.event_type),
                        event.rationale.clone().unwrap_or_else(|| "No rationale".to_string()),
                        event.sequence_number,
                        &event.hash.to_hex()[..16]
                    ),

                    // Structured data
                    "event": {
                        "id": event.id.to_string(),
                        "type": format!("{:?}", event.event_type),
                        "agent_id": event.agent_id.map(|id| id.to_string()),
                        "sequence": event.sequence_number,
                        "hash": event.hash.to_hex(),
                    },

                    // Actor attribution
                    "usr": match &event.actor {
                        ActorType::Human(name) => serde_json::json!({"name": name}),
                        ActorType::Bot(id) => serde_json::json!({"id": id.to_string(), "type": "bot"}),
                        ActorType::System => serde_json::json!({"type": "system"}),
                    },

                    // VEX custom attributes
                    "vex": {
                        "merkle_root": self.merkle_root.clone(),
                        "verified": self.verified,
                        "policy_version": event.policy_version.clone(),
                        "human_review_required": event.human_review_required,
                        "data_provenance_hash": event.data_provenance_hash.as_ref().map(|h| h.to_hex()),
                    },
                })
            })
            .collect()
    }

    /// Export all events to JSON Lines format (one JSON per line)
    /// Compatible with most log ingestion systems
    pub fn to_jsonl(&self) -> String {
        self.events
            .iter()
            .filter_map(|e| serde_json::to_string(e).ok())
            .collect::<Vec<_>>()
            .join("\n")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backend::MemoryBackend;

    #[tokio::test]
    async fn test_audit_store_isolation() {
        let backend = Arc::new(MemoryBackend::new());
        let store = AuditStore::new(backend);
        let t1 = "tenant-1";
        let t2 = "tenant-2";

        // Log to tenant 1
        store
            .log(
                t1,
                AuditEventType::AgentCreated,
                ActorType::System,
                None,
                serde_json::json!({}),
            )
            .await
            .unwrap();

        // Log to tenant 2
        store
            .log(
                t2,
                AuditEventType::AgentExecuted,
                ActorType::System,
                None,
                serde_json::json!({}),
            )
            .await
            .unwrap();

        // Verify isolation
        let chain1 = store.get_chain(t1).await.unwrap();
        let chain2 = store.get_chain(t2).await.unwrap();

        assert_eq!(chain1.len(), 1);
        assert_eq!(chain2.len(), 1);
        assert_ne!(chain1[0].id, chain2[0].id);

        let root1 = store
            .build_merkle_tree(t1)
            .await
            .unwrap()
            .root_hash()
            .cloned();
        let root2 = store
            .build_merkle_tree(t2)
            .await
            .unwrap()
            .root_hash()
            .cloned();
        assert_ne!(root1, root2);
    }
}
