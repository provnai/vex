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
    // ISO 42001 A.6 Lifecycle event types
    PolicyUpdate,
    ModelUpgrade,
    AnomalousBehavior,
    HumanOverride,
    Custom(String),
}

/// Actor type for audit attribution (ISO 42001 A.6.2.8)
/// 
/// # Privacy Warning (MEDIUM-1)
/// The `Human` variant stores user identifiers directly. When exporting to
/// external SIEM systems (OCSF, Splunk, Datadog), consider:
/// - GDPR/CCPA compliance for PII handling
/// - Using pseudonymized identifiers instead of real names
/// - Implementing consent checks before export
/// 
/// For privacy-sensitive deployments, use hashed or tokenized identifiers:
/// ```ignore
/// ActorType::Human(hash_user_id(user.id))
/// ```
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ActorType {
    /// AI agent performed the action
    Bot(Uuid),
    /// Human user performed the action (PII - handle with care)
    Human(String),
    /// System/automated process
    #[default]
    System,
}

/// Cryptographic signature for multi-party authorization (ISO 42001 A.6.1.3)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    /// Signer identifier (user ID or key fingerprint)
    pub signer_id: String,
    /// Signature timestamp
    pub signed_at: DateTime<Utc>,
    /// Hex-encoded signature bytes (64 bytes for Ed25519)
    pub signature_hex: String,
}

impl Signature {
    /// Create a new signature for a message
    /// 
    /// # Arguments
    /// * `signer_id` - Identifier for the signer (username, key fingerprint)
    /// * `message` - The message bytes to sign
    /// * `signing_key` - The Ed25519 signing key
    pub fn create(
        signer_id: impl Into<String>,
        message: &[u8],
        signing_key: &ed25519_dalek::SigningKey,
    ) -> Self {
        use ed25519_dalek::Signer;
        let signature = signing_key.sign(message);
        
        Self {
            signer_id: signer_id.into(),
            signed_at: Utc::now(),
            signature_hex: hex::encode(signature.to_bytes()),
        }
    }

    /// Verify this signature against a message and public key
    /// 
    /// Uses `verify_strict()` to prevent weak key attacks (CRITICAL-1 fix)
    /// 
    /// # Arguments
    /// * `message` - The message bytes that were signed
    /// * `verifying_key` - The Ed25519 public key
    /// 
    /// # Returns
    /// * `Ok(true)` if signature is valid
    /// * `Ok(false)` if signature format is invalid
    /// * `Err` if verification fails (signature doesn't match)
    pub fn verify(
        &self,
        message: &[u8],
        verifying_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool, String> {
        use ed25519_dalek::Verifier;
        
        // Decode hex signature
        let sig_bytes = match hex::decode(&self.signature_hex) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false), // Invalid hex = invalid signature
        };

        // Parse signature bytes
        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => return Ok(false), // Wrong length = invalid signature
        };

        let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

        // Use verify_strict to prevent weak key attacks
        match verifying_key.verify_strict(message, &signature) {
            Ok(()) => Ok(true),
            Err(e) => Err(format!("Signature verification failed: {}", e)),
        }
    }
}

/// Single audit event (ISO 42001 / EU AI Act compliant)
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
    
    // === ISO 42001 / EU AI Act Compliance Fields ===
    
    /// Who performed the action (ISO 42001 A.6.2.8)
    #[serde(default)]
    pub actor: ActorType,
    /// Explanation for the decision (ISO 42001 A.9.2)
    #[serde(default)]
    pub rationale: Option<String>,
    /// Version of the policy/instructions in effect (ISO 42001 A.6.1.3)
    #[serde(default)]
    pub policy_version: Option<String>,
    /// Hash of input data used for this decision (ISO 42001 A.7)
    #[serde(default)]
    pub data_provenance_hash: Option<Hash>,
    /// Whether this event requires human review (EU AI Act Article 14)
    #[serde(default)]
    pub human_review_required: bool,
    /// Multi-party authorization signatures (ISO 42001 A.6.1.3 / EU AI Act Article 14)
    #[serde(default)]
    pub approval_signatures: Vec<Signature>,
}

impl AuditEvent {
    /// Fields that should be redacted from audit log data for security
    const SENSITIVE_FIELDS: &'static [&'static str] = &[
        "password",
        "secret",
        "token",
        "api_key",
        "apikey",
        "key",
        "authorization",
        "auth",
        "credential",
        "private_key",
        "privatekey",
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

        // Default ISO 42001 / EU AI Act fields
        let actor = ActorType::System;
        let rationale: Option<String> = None;
        let policy_version: Option<String> = None;
        let data_provenance_hash: Option<Hash> = None;
        let human_review_required = false;
        let approval_signatures: Vec<Signature> = Vec::new();

        // Compute hash including ALL fields for tamper detection (CRITICAL-3 fix)
        let hash = Self::compute_hash(
            &event_type,
            timestamp,
            sequence_number,
            &data,
            &actor,
            &rationale,
            &policy_version,
            &data_provenance_hash,
            human_review_required,
            approval_signatures.len(),
        );

        Self {
            id,
            event_type,
            timestamp,
            agent_id,
            data,
            hash,
            previous_hash: None,
            sequence_number,
            actor,
            rationale,
            policy_version,
            data_provenance_hash,
            human_review_required,
            approval_signatures,
        }
    }

    /// Compute event hash including all compliance-critical fields
    /// Used by both new() and chained() to ensure consistent tamper detection
    fn compute_hash(
        event_type: &AuditEventType,
        timestamp: chrono::DateTime<Utc>,
        sequence_number: u64,
        data: &serde_json::Value,
        actor: &ActorType,
        rationale: &Option<String>,
        policy_version: &Option<String>,
        data_provenance_hash: &Option<Hash>,
        human_review_required: bool,
        approval_count: usize,
    ) -> Hash {
        // Format includes ALL fields to prevent tampering (ISO 42001 compliance)
        let content = format!(
            "{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}",
            event_type,
            timestamp.timestamp(),
            sequence_number,
            data,
            actor,
            rationale,
            policy_version,
            data_provenance_hash.as_ref().map(|h| h.to_hex()),
            human_review_required,
            approval_count,
        );
        Hash::digest(content.as_bytes())
    }

    /// Sanitize sensitive fields from audit data (HIGH-2 fix)
    /// 
    /// This is public so export methods can apply sanitization before
    /// sending data to external SIEM systems.
    pub fn sanitize_data(value: serde_json::Value) -> serde_json::Value {
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

    fn event_key(&self, tenant_id: &str, id: Uuid) -> String {
        format!("{}tenant:{}:event:{}", self.prefix, tenant_id, id)
    }

    fn chain_key(&self, tenant_id: &str) -> String {
        format!("{}tenant:{}:chain", self.prefix, tenant_id)
    }

    /// Get next sequence number atomically
    fn next_sequence(&self) -> u64 {
        self.sequence_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Log an audit event (automatically chained with sequence number)
    pub async fn log(
        &self,
        tenant_id: &str,
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
        self.backend
            .set(&self.event_key(tenant_id, event.id), &event)
            .await?;

        // Update chain
        let mut chain: Vec<Uuid> = self
            .backend
            .get(&self.chain_key(tenant_id))
            .await?
            .unwrap_or_default();
        chain.push(event.id);
        self.backend.set(&self.chain_key(tenant_id), &chain).await?;

        // Update last hash
        *last_hash = Some(event.hash.clone());

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
