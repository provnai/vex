//! Audit log types with Merkle verification (ISO 42001 / EU AI Act compliant)

use crate::merkle::Hash;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "algoswitch")]
use vex_algoswitch as algoswitch;

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
    /// CHORA Phase-2 Gate Decision (ALLOW/HALT/ESCALATE)
    GateDecision,
    Custom(String),
}

/// CHORA Evidence Capsule (RFC 8785 Compliant Metadata)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct EvidenceCapsule {
    pub capsule_id: String,
    pub outcome: String, // ALLOW, HALT, ESCALATE
    pub reason_code: String,
    pub sensors: serde_json::Value,
    pub reproducibility_context: serde_json::Value,
}

/// Actor type for audit attribution (ISO 42001 A.6.2.8)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub enum ActorType {
    /// AI agent performed the action
    Bot(Uuid),
    /// Human user performed the action
    Human(String),
    /// System/automated process
    #[default]
    System,
}

impl ActorType {
    /// Pseudonymize human actor ID using SHA-256 to protect PII (ISO 42001 A.6.2.8)
    pub fn pseudonymize(&self) -> Self {
        match self {
            Self::Human(id) => {
                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(id.as_bytes());
                Self::Human(hex::encode(hasher.finalize()))
            }
            other => other.clone(),
        }
    }
}

/// Cryptographic signature for multi-party authorization (ISO 42001 A.6.1.3)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Signature {
    pub signer_id: String,
    pub signed_at: DateTime<Utc>,
    pub signature_hex: String,
}

impl Signature {
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

    pub fn verify(
        &self,
        message: &[u8],
        verifying_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool, String> {
        let sig_bytes = match hex::decode(&self.signature_hex) {
            Ok(bytes) => bytes,
            Err(_) => return Ok(false),
        };

        let sig_array: [u8; 64] = match sig_bytes.try_into() {
            Ok(arr) => arr,
            Err(_) => return Ok(false),
        };

        let signature = ed25519_dalek::Signature::from_bytes(&sig_array);

        match verifying_key.verify_strict(message, &signature) {
            Ok(()) => Ok(true),
            Err(e) => Err(format!("Signature verification failed: {}", e)),
        }
    }
}

/// Single audit event (ISO 42001 / EU AI Act compliant)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub event_type: AuditEventType,
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<Uuid>,
    pub data: serde_json::Value,
    pub hash: Hash,
    pub previous_hash: Option<Hash>,
    pub sequence_number: u64,

    // Compliance Fields
    pub actor: ActorType,
    pub rationale: Option<String>,
    pub policy_version: Option<String>,
    pub data_provenance_hash: Option<Hash>,
    pub human_review_required: bool,
    pub approval_signatures: Vec<Signature>,

    // CHORA Alignment
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_capsule: Option<EvidenceCapsule>,
}

/// Parameters for consistent event hashing
#[derive(Serialize)]
pub struct HashParams<'a> {
    pub event_type: &'a AuditEventType,
    pub timestamp: i64, // Use timestamp for JCS stability
    pub sequence_number: u64,
    pub data: &'a serde_json::Value,
    pub actor: &'a ActorType,
    pub rationale: &'a Option<String>,
    pub policy_version: &'a Option<String>,
    pub data_provenance_hash: &'a Option<Hash>,
    pub human_review_required: bool,
    pub approval_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_capsule: &'a Option<EvidenceCapsule>,
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
        let evidence_capsule: Option<EvidenceCapsule> = None;

        // Compute hash including ALL fields (Centralized in vex-core)
        let hash = Self::compute_hash(HashParams {
            event_type: &event_type,
            timestamp: timestamp.timestamp(),
            sequence_number,
            data: &data,
            actor: &actor,
            rationale: &rationale,
            policy_version: &policy_version,
            data_provenance_hash: &data_provenance_hash,
            human_review_required,
            approval_count: approval_signatures.len(),
            evidence_capsule: &evidence_capsule,
        });

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
            evidence_capsule,
        }
    }

    /// Sanitize sensitive fields from audit data (HIGH-2 fix)
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
        // Rehash including previous hash and sequence (Centralized in vex-core)
        event.hash = Self::compute_chained_hash(&event.hash, &previous_hash, sequence_number);
        event
    }

    /// Hashing using RFC 8785 (JCS) for cross-platform determinism
    pub fn compute_hash(params: HashParams) -> Hash {
        match serde_jcs::to_vec(&params) {
            Ok(jcs_bytes) => Hash::digest(&jcs_bytes),
            Err(_) => {
                // Fallback (should not happen if HashParams is simple)
                let content = format!(
                    "{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}",
                    params.event_type,
                    params.timestamp,
                    params.sequence_number,
                    params.data,
                    params.actor,
                    params.rationale,
                    params.policy_version,
                    params.data_provenance_hash.as_ref().map(|h| h.to_hex()),
                    params.human_review_required,
                    params.approval_count,
                );
                Hash::digest(content.as_bytes())
            }
        }
    }

    pub fn compute_chained_hash(base_hash: &Hash, prev_hash: &Hash, sequence: u64) -> Hash {
        let content = format!("{}:{}:{}", base_hash, prev_hash, sequence);
        Hash::digest(content.as_bytes())
    }

    /// Optimized hash computation using AlgoSwitch for non-critical performance tracing
    #[cfg(feature = "algoswitch")]
    pub fn compute_optimized_hash(params: HashParams) -> u64 {
        match serde_jcs::to_vec(&params) {
            Ok(jcs_bytes) => algoswitch::select_hash(&jcs_bytes).0,
            Err(_) => {
                let content = format!(
                    "{:?}:{}:{}:{:?}:{:?}:{:?}:{}",
                    params.event_type,
                    params.timestamp,
                    params.sequence_number,
                    params.data,
                    params.actor,
                    params.rationale,
                    params.approval_count,
                );
                algoswitch::select_hash(content.as_bytes()).0
            }
        }
    }
}
