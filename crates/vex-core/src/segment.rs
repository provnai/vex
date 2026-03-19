//! # VEX Segments
//!
//! Provides the data structures and JCS canonicalization for the v0.1.0 "Hardened" Commitment model.

use crate::merkle::Hash;
use crate::zk::{ZkError, ZkVerifier};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Intent Data (VEX Pillar)
/// Proves the proposed action before execution. It supports two variants:
/// - Transparent: Standard human-readable reasoning (Standard).
/// - Shadow: STARK-proofed hidden intent for privacy (High-Compliance).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(untagged, rename_all = "snake_case")]
pub enum IntentData {
    Transparent {
        request_sha256: String,
        confidence: f64,
        #[serde(default)]
        capabilities: Vec<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        magpie_source: Option<String>,

        /// Catch-all for extra fields to preserve binary parity in JCS.
        #[serde(flatten, default)]
        metadata: serde_json::Value,
    },
    Shadow {
        commitment_hash: String,
        stark_proof_b64: String,
        public_inputs: serde_json::Value,

        /// New Phase 2: Plonky3 Circuit Identity
        #[serde(skip_serializing_if = "Option::is_none")]
        circuit_id: Option<String>,

        /// Catch-all for extra fields to preserve binary parity in JCS.
        #[serde(flatten, default)]
        metadata: serde_json::Value,
    },
}

impl IntentData {
    pub fn to_jcs_hash(&self) -> Result<Hash, String> {
        let jcs_bytes =
            serde_jcs::to_vec(self).map_err(|e| format!("JCS serialization failed: {}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(&jcs_bytes);
        let result = hasher.finalize();

        Ok(Hash::from_bytes(result.into()))
    }

    /// Verifies the Zero-Knowledge proof for Shadow intents.
    /// For Transparent intents, this always returns Ok(true).
    pub fn verify_shadow(&self, verifier: &dyn ZkVerifier) -> Result<bool, ZkError> {
        match self {
            IntentData::Transparent { .. } => Ok(true),
            IntentData::Shadow {
                commitment_hash,
                stark_proof_b64,
                public_inputs,
                ..
            } => verifier.verify_stark(commitment_hash, stark_proof_b64, public_inputs),
        }
    }
}

/// Authority Data (CHORA Pillar)
/// Proves the governance decision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorityData {
    pub capsule_id: String,
    pub outcome: String,
    pub reason_code: String,
    pub trace_root: String,
    pub nonce: u64,

    /// New Phase 2: CHORA Binding Mode Fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<ContinuationToken>,

    #[serde(
        default = "default_sensor_value",
        skip_serializing_if = "serde_json::Value::is_null"
    )]
    pub gate_sensors: serde_json::Value,

    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: serde_json::Value,
}

fn default_sensor_value() -> serde_json::Value {
    serde_json::Value::Null
}

/// Witness Data (CHORA Append-Only Log)
/// Proves the receipt issuance parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessData {
    pub chora_node_id: String,
    pub receipt_hash: String,
    pub timestamp: u64,
    /// Diagnostic or display-only fields that are NOT part of the commitment surface.
    #[serde(flatten, default)]
    pub metadata: serde_json::Value,
}

impl WitnessData {
    /// Compute the "witness_hash" using the v0.3 Minimal Witness spec.
    /// ONLY chora_node_id and timestamp are committed.
    /// receipt_hash is post-seal metadata and is NOT part of the witness commitment surface.
    /// Ref: CHORA_VERIFICATION_CONTRACT_v0.3.md
    pub fn to_commitment_hash(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct MinimalWitness<'a> {
            chora_node_id: &'a str,
            timestamp: u64,
        }

        let minimal = MinimalWitness {
            chora_node_id: &self.chora_node_id,
            timestamp: self.timestamp,
        };

        let jcs_bytes = serde_jcs::to_vec(&minimal)
            .map_err(|e| format!("JCS serialization of minimal witness failed: {}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(&jcs_bytes);
        Ok(hex::encode(hasher.finalize()))
    }

    pub fn to_jcs_hash(&self) -> Result<Hash, String> {
        let hex = self.to_commitment_hash()?;
        Ok(Hash::from_bytes(
            hex::decode(hex)
                .map_err(|e| e.to_string())?
                .try_into()
                .map_err(|_| "Invalid hash length")?,
        ))
    }
}

/// Identity Data (Attest Pillar)
/// Proves the silicon source and its integrity state.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityData {
    pub aid: String,
    pub identity_type: String,
    /// Platform Configuration Registers (PCRs) for hardware-rooted integrity.
    /// Map of PCR index (e.g., 0, 7, 11) to SHA-256 hash (hex string).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcrs: Option<std::collections::HashMap<u32, String>>,

    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: serde_json::Value,
}

/// Continuation Token (Phase 2 Enforcement Primitive)
/// A signed artifact that permits execution after an escalation.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct ContinuationToken {
    pub payload: ContinuationPayload,
    pub signature: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
pub struct ContinuationPayload {
    pub schema: String,
    pub ledger_event_id: String,
    pub source_capsule_root: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_event_id: Option<String>,
    pub nonce: String,
    pub iat: String,
    pub exp: String,
    pub issuer: String,
}

impl ContinuationPayload {
    /// Validates the token's lifecycle (iat/exp) with a grace period.
    pub fn validate_lifecycle(&self, now: chrono::DateTime<chrono::Utc>) -> Result<(), String> {
        let iat = chrono::DateTime::parse_from_rfc3339(&self.iat)
            .map_err(|e| format!("Invalid iat: {}", e))?
            .with_timezone(&chrono::Utc);
        let exp = chrono::DateTime::parse_from_rfc3339(&self.exp)
            .map_err(|e| format!("Invalid exp: {}", e))?
            .with_timezone(&chrono::Utc);

        let leeway = chrono::Duration::seconds(30);

        if now < iat - leeway {
            return Err("Token issued in the future (beyond leeway)".to_string());
        }

        if now > exp + leeway {
            return Err("Token expired (beyond leeway)".to_string());
        }

        Ok(())
    }
}

impl ContinuationToken {
    /// Computes the JCS hash of the payload for signature verification.
    pub fn payload_hash(&self) -> Result<Vec<u8>, String> {
        let jcs_bytes = serde_jcs::to_vec(&self.payload)
            .map_err(|e| format!("JCS serialization failed: {}", e))?;
        let mut hasher = sha2::Sha256::new();
        use sha2::Digest;
        hasher.update(&jcs_bytes);
        Ok(hasher.finalize().to_vec())
    }
}

/// Crypto verification details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CryptoData {
    pub algo: String,
    pub public_key_endpoint: String,
    pub signature_scope: String,
    pub signature_b64: String,
}

/// Auditability metadata to link the raw payload to the intent.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RequestCommitment {
    pub canonicalization: String,
    pub payload_sha256: String,
    pub payload_encoding: String,
}

/// A Composite Evidence Capsule (The v0.1.0 "Zero-Trust Singularity" Root)
/// Binds Intent, Authority, Identity, and Witness into a single commitment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Capsule {
    pub capsule_id: String,
    /// VEX Pillar: What was intended
    pub intent: IntentData,
    /// CHORA Pillar: Who authorized it
    pub authority: AuthorityData,
    /// ATTEST Pillar: Where it executed (Silicon)
    pub identity: IdentityData,
    /// CHORA Log Pillar: Where the receipt lives
    pub witness: WitnessData,

    // Derived hashes for transparency
    pub intent_hash: String,
    pub authority_hash: String,
    pub identity_hash: String,
    pub witness_hash: String,
    pub capsule_root: String,

    /// Ed25519 signature details
    pub crypto: CryptoData,

    /// Optional auditable link to raw payload (v0.2+)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_commitment: Option<RequestCommitment>,
}

impl Capsule {
    /// Compute the canonical "capsule_root" using the Binary Merkle Tree model.
    /// This enables ZK-Explorer partial disclosure proofs for "Shadow Intents".
    pub fn to_composite_hash(&self) -> Result<Hash, String> {
        let intent_h = self.intent.to_jcs_hash()?;

        // Authority and Identity are hashed as "Naked" leaves for byte-level interop with CHORA.
        let authority_h = {
            let jcs = serde_jcs::to_vec(&self.authority).map_err(|e| e.to_string())?;
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(&jcs);
            Hash::from_bytes(hasher.finalize().into())
        };

        let identity_h = {
            let jcs = serde_jcs::to_vec(&self.identity).map_err(|e| e.to_string())?;
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(&jcs);
            Hash::from_bytes(hasher.finalize().into())
        };

        let witness_h = self.witness.to_jcs_hash()?;

        // Build 4-leaf Merkle Tree (RFC 6962 compatible structure)
        let leaves = vec![
            ("intent".to_string(), intent_h),
            ("authority".to_string(), authority_h),
            ("identity".to_string(), identity_h),
            ("witness".to_string(), witness_h),
        ];

        let tree = crate::merkle::MerkleTree::from_leaves(leaves);

        tree.root_hash()
            .cloned()
            .ok_or_else(|| "Failed to calculate Merkle root".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_segment_jcs_deterministic() {
        let segment1 = IntentData::Transparent {
            request_sha256: "8ee6010d905547c377c67e63559e989b8073b168f11a1ffefd092c7ca962076e"
                .to_string(),
            confidence: 0.95,
            capabilities: vec![],
            magpie_source: None,
            metadata: serde_json::Value::Null,
        };
        let segment2 = segment1.clone();

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_eq!(hash1, hash2, "JCS hashing must be deterministic");
    }

    #[test]
    fn test_intent_segment_content_change() {
        let segment1 = IntentData::Transparent {
            request_sha256: "a".into(),
            confidence: 0.5,
            capabilities: vec![],
            magpie_source: None,
            metadata: serde_json::Value::Null,
        };
        let mut segment2 = segment1.clone();
        if let IntentData::Transparent {
            ref mut confidence, ..
        } = segment2
        {
            *confidence = 0.9;
        }

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_ne!(hash1, hash2, "Hashes must change when content changes");
    }

    #[test]
    fn test_shadow_intent_jcs_deterministic() {
        let segment1 = IntentData::Shadow {
            commitment_hash: "5555555555555555555555555555555555555555555555555555555555555555"
                .to_string(),
            stark_proof_b64: "c29tZS1zdGFyay1wcm9vZg==".to_string(),
            public_inputs: serde_json::json!({
                "policy_id": "standard-v1",
                "outcome_commitment": "ALLOW"
            }),
            circuit_id: None,
            metadata: serde_json::Value::Null,
        };
        let segment2 = segment1.clone();

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_eq!(hash1, hash2, "Shadow JCS hashing must be deterministic");

        // Verify JCS serialization (untagged)
        let jcs_bytes = serde_jcs::to_vec(&segment1).unwrap();
        let jcs_str = String::from_utf8(jcs_bytes).unwrap();
        assert!(
            jcs_str.contains("\"commitment_hash\""),
            "JCS must include the commitment_hash"
        );
    }

    #[test]
    fn test_witness_metadata_exclusion() {
        let base_witness = WitnessData {
            chora_node_id: "node-1".to_string(),
            receipt_hash: "hash-1".to_string(),
            timestamp: 1710396000,
            metadata: serde_json::Value::Null,
        };

        let hash_base = base_witness.to_commitment_hash().unwrap();

        let mut metadata_witness = base_witness.clone();
        metadata_witness.metadata = serde_json::json!({
            "witness_mode": "sentinel",
            "diagnostics": {
                "latency_ms": 42
            }
        });

        let hash_with_metadata = metadata_witness.to_commitment_hash().unwrap();

        assert_eq!(
            hash_base, hash_with_metadata,
            "Witness hash must be invariant to extra metadata fields"
        );
    }

    #[test]
    fn test_witness_segment_minimal_interop() {
        // v0.3 spec: witness commitment = {chora_node_id, timestamp} ONLY.
        // receipt_hash is post-seal metadata and is excluded.
        // Canonical JCS surface: {"chora_node_id":"chora-gate-v1","timestamp":1710396000}
        let witness = WitnessData {
            chora_node_id: "chora-gate-v1".to_string(),
            receipt_hash: "ignored-in-v03".to_string(),
            timestamp: 1710396000,
            metadata: serde_json::json!({
                "witness_mode": "full",
                "observational_only": false
            }),
        };

        let hash_hex = witness.to_commitment_hash().expect("Hashing failed");

        // SHA256({"chora_node_id":"chora-gate-v1","timestamp":1710396000})
        assert_eq!(
            hash_hex, "84b8cc23c2d510d30920e3200913f45cf0097365f2fd377e1de1b3d831b5b9ec",
            "v0.3 witness hash must exclude receipt_hash"
        );
    }

    #[test]
    fn test_authority_extra_fields_parity() {
        // Specimen based on the CHORA example
        let json_data = serde_json::json!({
            "capsule_id": "example-capsule-001",
            "outcome": "ALLOW",
            "reason_code": "policy_ok",
            "trace_root": "trace-001",
            "nonce": 1234567890,
            "gate_sensors": null,
            "rule_set_owner": "chora-authority-node",
            "fail_closed": true
        });

        let authority: AuthorityData = serde_json::from_value(json_data.clone()).unwrap();

        // Ensure extra fields went into metadata
        assert_eq!(authority.metadata["rule_set_owner"], "chora-authority-node");
        assert_eq!(authority.metadata["fail_closed"], true);

        // Verify JCS serialization includes the extra fields
        let jcs_bytes = serde_jcs::to_vec(&authority).unwrap();
        let jcs_str = String::from_utf8(jcs_bytes).unwrap();

        assert!(jcs_str.contains("\"rule_set_owner\":\"chora-authority-node\""));
        assert!(jcs_str.contains("\"fail_closed\":true"));
    }

    #[test]
    fn test_chora_live_specimen_parity() {
        // v0.3 live specimen from George's canonical bundle (2026-03-19T08:34:21Z)
        // capsule_id: 1a3b2267-23a3-46d2-b34e-138912b80652
        // v0.3 witness spec: {chora_node_id, timestamp} ONLY
        let witness = WitnessData {
            chora_node_id: "chora-vps-1".to_string(),
            receipt_hash: "5bfc2b79f9bab22abd12a196aafd8a91cdb14c2cf68230375de9569d99236b5c".to_string(),
            timestamp: 1773909261,
            metadata: serde_json::json!({
                "witness_mode": "attached",
                "sentinel_mode": "observe_only",
                "observational_only": true
            }),
        };

        // v0.3: only chora_node_id + timestamp committed
        let witness_hash = witness.to_commitment_hash().unwrap();
        assert_eq!(
            witness_hash, "98d67e7ef952956fae8b75a907423dcb8856af61672ac3c95a11d76af6bd7f25",
            "v0.3 witness hash must match George's canonical bundle (receipt_hash excluded)"
        );

        // Capsule root from George's canonical bundle — verified via RFC 6962 Merkle tree.
        // Tree: combine(combine(intent,authority), combine(identity,witness))
        // where combine(a,b) = SHA256(0x01 || a_bytes || b_bytes)
        let intent_h   = hex::decode("e26f0ce40a2434a0a2cb506fbd21415c5aa398fe1bff0c5fa72872afa9dedbfa").unwrap();
        let authority_h = hex::decode("76d0b70f4f0d2df0dd538ce5e03e6eab8c418d949e6b83bafac7da97be6d5a27").unwrap();
        let identity_h  = hex::decode("9aa0bb3fcf0a1cac6794b79cf138dca1a65d3773c63fe4891d9efe0466ff313e").unwrap();
        let witness_h   = hex::decode("98d67e7ef952956fae8b75a907423dcb8856af61672ac3c95a11d76af6bd7f25").unwrap();

        fn merkle_combine(left: &[u8], right: &[u8]) -> Vec<u8> {
            use sha2::Digest;
            let mut h = sha2::Sha256::new();
            h.update([0x01u8]);
            h.update(left);
            h.update(right);
            h.finalize().to_vec()
        }

        let l1_left  = merkle_combine(&intent_h, &authority_h);
        let l1_right = merkle_combine(&identity_h, &witness_h);
        let root     = merkle_combine(&l1_left, &l1_right);
        let capsule_root = hex::encode(&root);

        assert_eq!(
            capsule_root, "d7d45e03f3d5a4e6a2af94b033a8724cad986ec779282163fc2a4b5ff90a1bc4",
            "Capsule root must match George's canonical v0.3 bundle (RFC 6962 Merkle)"
        );
    }
}
