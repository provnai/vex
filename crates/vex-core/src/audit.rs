//! Audit log types with Merkle verification (ISO 42001 / EU AI Act compliant)

use crate::merkle::Hash;
use crate::segment::SchemaValue;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[cfg(feature = "algoswitch")]
use vex_algoswitch as algoswitch;

/// Audit event types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
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
    GenomeEvolved,
    AnomalousBehavior,
    HumanOverride,
    /// CHORA Phase-2 Gate Decision
    #[serde(rename = "CHORA_GATE_DECISION")]
    GateDecision,
    /// Phase 2: AI Escalation for Human Review
    Escalation,
    #[serde(untagged)]
    Custom(String),
}

/// CHORA Evidence Capsule (RFC 8785 Compliant Metadata)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, utoipa::ToSchema)]
pub struct EvidenceCapsule {
    pub capsule_id: String,
    pub outcome: String, // ALLOW, HALT, ESCALATE
    pub reason_code: String,
    pub witness_receipt: String,
    pub nonce: u64,
    /// Bundled Magpie AST for independent verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magpie_source: Option<String>,
    pub gate_sensors: SchemaValue,
    pub reproducibility_context: SchemaValue,

    /// New Phase 2: Coordination Ledger resolution link
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_vep_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<crate::segment::ContinuationToken>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_data: Option<crate::segment::IntentData>,

    /// Optional full VEP binary blob
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vep_blob: Option<Vec<u8>>,
}

/// Actor type for audit attribution (ISO 42001 A.6.2.8)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "type", content = "id", rename_all = "lowercase")]
pub enum ActorType {
    /// AI agent performed the action
    Bot(Uuid),
    /// Human user performed the action
    Human(String),
    /// System/automated process
    System(String),
}

impl Default for ActorType {
    fn default() -> Self {
        ActorType::System("vex_core".to_string())
    }
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
    /// Optional full VEP binary blob for independent verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub vep_blob: Option<Vec<u8>>,
    pub schema_version: String,
}

/// Parameters for consistent event hashing
#[derive(Serialize)]
pub struct HashParams<'a> {
    pub event_type: &'a AuditEventType,
    pub timestamp: i64, // Use timestamp for JCS stability
    pub sequence_number: u64,
    pub data: &'a serde_json::Value,
    pub actor: &'a ActorType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rationale: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub policy_version: &'a Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_provenance_hash: &'a Option<Hash>,
    pub human_review_required: bool,
    pub approval_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub evidence_capsule: &'a Option<EvidenceCapsule>,
    pub schema_version: &'a str,
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
        let actor = ActorType::System("vex_core".to_string());
        let rationale: Option<String> = None;
        let policy_version: Option<String> = None;
        let data_provenance_hash: Option<Hash> = None;
        let human_review_required = false;
        let approval_signatures: Vec<Signature> = Vec::new();
        let evidence_capsule: Option<EvidenceCapsule> = None;
        let schema_version = "1.0".to_string();

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
            schema_version: &schema_version,
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
            vep_blob: None,
            schema_version,
        }
    }

    /// Patterns that indicate a secret value (checked against string values)
    const SECRET_VALUE_PREFIXES: &'static [&'static str] = &[
        "sk-",     // OpenAI/Stripe keys
        "ghp_",    // GitHub personal tokens
        "gho_",    // GitHub OAuth tokens
        "Bearer ", // Auth bearer tokens
        "Basic ",  // Basic auth
        "whsec_",  // Webhook secrets
        "xoxb-",   // Slack bot tokens
        "xoxp-",   // Slack user tokens
    ];

    /// Check if a string value looks like a secret
    fn is_secret_value(s: &str) -> bool {
        // Check known prefixes
        if Self::SECRET_VALUE_PREFIXES.iter().any(|p| s.starts_with(p)) {
            return true;
        }
        // Check for base64-encoded blobs >= 32 chars (likely secrets)
        if s.len() >= 32
            && s.chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '+' || c == '/' || c == '=')
        {
            return true;
        }
        false
    }

    /// Sanitize sensitive fields and values from audit data (HIGH-2 fix)
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
            serde_json::Value::String(s) if Self::is_secret_value(&s) => {
                serde_json::Value::String("[REDACTED]".to_string())
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
                // Fallback with domain separation prefix
                let content = format!(
                    "vex-audit-v1:{:?}:{}:{}:{:?}:{:?}:{:?}:{:?}:{:?}:{}:{}:{}",
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
                    params.schema_version,
                );
                Hash::digest(content.as_bytes())
            }
        }
    }

    pub fn compute_chained_hash(base_hash: &Hash, prev_hash: &Hash, sequence: u64) -> Hash {
        let content = format!("vex-chain-v1:{}:{}:{}", base_hash, prev_hash, sequence);
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

    /// Sign the evidence capsule payload using the hardware-rooted TPM identity.
    /// Uses JCS serialization for deterministic CHORA compliance.
    pub async fn sign_hardware(
        &mut self,
        agent_identity: &vex_hardware::api::AgentIdentity,
    ) -> Result<(), String> {
        let params = HashParams {
            event_type: &self.event_type,
            timestamp: self.timestamp.timestamp(),
            sequence_number: self.sequence_number,
            data: &self.data,
            actor: &self.actor,
            rationale: &self.rationale,
            policy_version: &self.policy_version,
            data_provenance_hash: &self.data_provenance_hash,
            human_review_required: self.human_review_required,
            approval_count: self.approval_signatures.len(),
            evidence_capsule: &self.evidence_capsule,
            schema_version: &self.schema_version,
        };

        // 1. Serialize parameters to JCS bytes deterministically
        let jcs_bytes =
            serde_jcs::to_vec(&params).map_err(|e| format!("JCS serialization failed: {}", e))?;

        // 2 & 3. Generate the signature directly over the JCS bytes using hardware-rooted identity
        let raw_signature_bytes = agent_identity.sign(&jcs_bytes);

        // 4. Wrap the raw signature in VEX's unified Signature tracking format
        let final_signature = Signature {
            signer_id: agent_identity.agent_id.clone(),
            signed_at: Utc::now(),
            signature_hex: hex::encode(raw_signature_bytes),
        };

        self.approval_signatures.push(final_signature);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_sanitize_sensitive_keys() {
        let data = json!({
            "name": "test",
            "api_key": "sk-abc123",
            "nested": {
                "password": "secret123",
                "safe": "value"
            }
        });
        let sanitized = AuditEvent::sanitize_data(data);
        assert_eq!(sanitized["api_key"], "[REDACTED]");
        assert_eq!(sanitized["nested"]["password"], "[REDACTED]");
        assert_eq!(sanitized["nested"]["safe"], "value");
        assert_eq!(sanitized["name"], "test");
    }

    #[test]
    fn test_sanitize_secret_values() {
        let data = json!({
            "some_field": "sk-1234567890abcdef",
            "token_field": "ghp_abcdefghijklmnop",
            "bearer_field": "Bearer eyJhbGciOiJIUz",
            "safe_field": "hello world"
        });
        let sanitized = AuditEvent::sanitize_data(data);
        assert_eq!(sanitized["some_field"], "[REDACTED]");
        assert_eq!(sanitized["token_field"], "[REDACTED]");
        assert_eq!(sanitized["bearer_field"], "[REDACTED]");
        assert_eq!(sanitized["safe_field"], "hello world");
    }

    #[test]
    fn test_sanitize_nested_arrays() {
        let data = json!({
            "items": [
                {"password": "secret"},
                {"name": "safe"}
            ]
        });
        let sanitized = AuditEvent::sanitize_data(data);
        assert_eq!(sanitized["items"][0]["password"], "[REDACTED]");
        assert_eq!(sanitized["items"][1]["name"], "safe");
    }

    #[test]
    fn test_chained_hash_domain_separation() {
        let hash1 = Hash::digest(b"event1");
        let hash2 = Hash::digest(b"event2");
        let chained = AuditEvent::compute_chained_hash(&hash1, &hash2, 1);
        // Should be different from non-domain-separated version
        let raw = Hash::digest(format!("{}:{}:{}", hash1, hash2, 1).as_bytes());
        assert_ne!(chained, raw, "Domain-separated hash should differ from raw");
    }

    #[tokio::test]
    async fn test_hardware_signature() {
        // Force fallback mode for the test (no physical TPM needed)
        std::env::set_var("VEX_HARDWARE_ATTESTATION", "false");

        // 1. Initialize the mock/fallback hardware keystore
        let keystore = vex_hardware::api::HardwareKeystore::new()
            .await
            .expect("Failed to initialize keystore");

        // 2. Generate a random identity
        // In fallback mode, get_identity requires a seed blob. Let's create a dummy encrypted blob (fallback provider expects raw 32 bytes)
        let dummy_seed = [42u8; 32];
        let encrypted_blob = keystore
            .seal_identity(&dummy_seed)
            .await
            .expect("Failed to seal");
        let agent_identity = keystore
            .get_identity(&encrypted_blob)
            .await
            .expect("Failed to get identity");

        // 3. Create a sample audit event
        let mut event = AuditEvent::new(
            AuditEventType::GateDecision,
            Some(Uuid::new_v4()),
            json!({"status": "approved"}),
            1,
        );

        // 4. Sign it using the hardware identity
        assert!(
            event.approval_signatures.is_empty(),
            "Event should start with no signatures"
        );

        let sign_result = event.sign_hardware(&agent_identity).await;
        assert!(sign_result.is_ok(), "Signing should succeed");

        // 5. Verify the signature was attached correctly
        assert_eq!(
            event.approval_signatures.len(),
            1,
            "One signature should be appended"
        );

        let sig = &event.approval_signatures[0];
        assert_eq!(
            sig.signer_id, agent_identity.agent_id,
            "Signer ID should match agent identity"
        );
        assert!(
            !sig.signature_hex.is_empty(),
            "Signature hex should not be empty"
        );
    }

    #[tokio::test]
    async fn test_hardware_signature_deterministic() {
        std::env::set_var("VEX_HARDWARE_ATTESTATION", "false");
        let keystore = vex_hardware::api::HardwareKeystore::new().await.unwrap();
        let dummy_seed = [42u8; 32];
        let encrypted_blob = keystore.seal_identity(&dummy_seed).await.unwrap();
        let agent_identity = keystore.get_identity(&encrypted_blob).await.unwrap();

        let mut event1 = AuditEvent::new(
            AuditEventType::GateDecision,
            Some(Uuid::new_v4()),
            json!({"status": "approved"}),
            1,
        );
        // Force timestamps and UUIDs to be identical for deterministic test
        let id = Uuid::new_v4();
        let ts = Utc::now();
        event1.id = id;
        event1.timestamp = ts;

        let mut event2 = event1.clone();

        event1.sign_hardware(&agent_identity).await.unwrap();
        event2.sign_hardware(&agent_identity).await.unwrap();

        assert_eq!(
            event1.approval_signatures[0].signature_hex,
            event2.approval_signatures[0].signature_hex,
            "Signatures for identical payloads must be deterministically equal"
        );
    }

    #[tokio::test]
    async fn test_hardware_signature_tamper_evident() {
        std::env::set_var("VEX_HARDWARE_ATTESTATION", "false");
        let keystore = vex_hardware::api::HardwareKeystore::new().await.unwrap();
        let dummy_seed = [42u8; 32];
        let encrypted_blob = keystore.seal_identity(&dummy_seed).await.unwrap();
        let agent_identity = keystore.get_identity(&encrypted_blob).await.unwrap();

        let mut event1 = AuditEvent::new(
            AuditEventType::GateDecision,
            Some(Uuid::new_v4()),
            json!({"status": "approved"}),
            1,
        );
        let mut event2 = event1.clone();

        // Tamper with the payload of event2
        event2.data = json!({"status": "denied"});

        event1.sign_hardware(&agent_identity).await.unwrap();
        event2.sign_hardware(&agent_identity).await.unwrap();

        assert_ne!(
            event1.approval_signatures[0].signature_hex,
            event2.approval_signatures[0].signature_hex,
            "Signatures must change completely if the payload is tampered with"
        );
    }

    #[tokio::test]
    async fn test_hardware_signature_raw_dalek_verification() {
        use ed25519_dalek::{Signature as DalekSignature, Verifier};

        std::env::set_var("VEX_HARDWARE_ATTESTATION", "false");
        let keystore = vex_hardware::api::HardwareKeystore::new().await.unwrap();
        let dummy_seed = [42u8; 32];
        let encrypted_blob = keystore.seal_identity(&dummy_seed).await.unwrap();
        let agent_identity = keystore.get_identity(&encrypted_blob).await.unwrap();

        let mut event = AuditEvent::new(
            AuditEventType::GateDecision,
            Some(Uuid::new_v4()),
            json!({"status": "approved"}),
            1,
        );
        event.sign_hardware(&agent_identity).await.unwrap();

        let sig_hex = &event.approval_signatures[0].signature_hex;
        let sig_bytes = hex::decode(sig_hex).unwrap();
        let sig_array: [u8; 64] = sig_bytes.try_into().unwrap();
        let dalek_sig = DalekSignature::from_bytes(&sig_array);

        // Reconstruct exactly what JCS bytes were signed
        let params = HashParams {
            event_type: &event.event_type,
            timestamp: event.timestamp.timestamp(),
            sequence_number: event.sequence_number,
            data: &event.data,
            actor: &event.actor,
            rationale: &event.rationale,
            policy_version: &event.policy_version,
            data_provenance_hash: &event.data_provenance_hash,
            human_review_required: event.human_review_required,
            approval_count: 0, // It was 0 when signed
            evidence_capsule: &event.evidence_capsule,
            schema_version: &event.schema_version,
        };
        let expected_jcs_bytes = serde_jcs::to_vec(&params).unwrap();

        // Regenerate the signing key strictly to get its verifying (public) key
        // Note: For a real TPM, you wouldn't be able to extract the private seed,
        // but since we are in fallback mock mode for testing, we can reconstruct the verifying key from the dummy seed.
        let signing_key = ed25519_dalek::SigningKey::from_bytes(&dummy_seed);
        let verifying_key = signing_key.verifying_key();

        // Verify the signature against the JCS payload using the public key
        assert!(
            verifying_key.verify(&expected_jcs_bytes, &dalek_sig).is_ok(),
            "Raw Dalek verification failed: The generated signature does not mathematically match the JCS payload."
        );
    }
}
