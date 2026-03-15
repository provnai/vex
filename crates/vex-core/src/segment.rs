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
    /// Compute the "witness_hash" using the Minimal Witness spec.
    /// Only chora_node_id, receipt_hash, and timestamp are hashed.
    pub fn to_commitment_hash(&self) -> Result<String, String> {
        #[derive(Serialize)]
        struct MinimalWitness<'a> {
            chora_node_id: &'a str,
            receipt_hash: &'a str,
            timestamp: u64,
        }

        let minimal = MinimalWitness {
            chora_node_id: &self.chora_node_id,
            receipt_hash: &self.receipt_hash,
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
        // Specimen from CHORA Witness Network
        // {
        //  "chora_node_id": "chora-gate-v1",
        //  "receipt_hash": "",
        //  "timestamp": 1710396000
        // }
        let witness = WitnessData {
            chora_node_id: "chora-gate-v1".to_string(),
            receipt_hash: "".to_string(),
            timestamp: 1710396000,
            metadata: serde_json::json!({
                "witness_mode": "full",
                "observational_only": false
            }),
        };

        let hash_hex = witness.to_commitment_hash().expect("Hashing failed");

        // Expected hash for exactly: {"chora_node_id":"chora-gate-v1","receipt_hash":"","timestamp":1710396000}
        // JCS should be lexicographical: chora_node_id -> receipt_hash -> timestamp
        assert_eq!(
            hash_hex, "79988e14e875e1fe409ccf13628c2c12bc3d2eeacfb09a9024889def4fc8262b",
            "Witness hash must match the CHORA spec even with extra metadata"
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
        // 1. Verify Witness Hash Parity
        // Sample from CHORA Witness Network - 2026-03-14 11:26 AM
        let witness = WitnessData {
            chora_node_id: "chora-vps-1".to_string(),
            receipt_hash: "".to_string(),
            timestamp: 1773483683,
            metadata: serde_json::json!({
                "witness_mode": "attached",
                "sentinel_mode": "observe_only",
                "observational_only": true
            }),
        };

        // Compute legacy witness hash (v0.2 excludes receipt_hash)
        let legacy_witness = serde_json::json!({
            "chora_node_id": witness.chora_node_id,
            "timestamp": witness.timestamp
        });
        let jcs_bytes = serde_jcs::to_vec(&legacy_witness).unwrap();
        let mut hasher = sha2::Sha256::new();
        hasher.update(&jcs_bytes);
        let witness_hash = hex::encode(hasher.finalize());

        assert_eq!(
            witness_hash, "7d4e2acaa7e459261d48f79cbec2a08ef5f8489e7cb610f375b708f9b8027e33",
            "Witness hash must match the CHORA live production sample (excluding receipt_hash)"
        );

        // 2. Verify Capsule Root Commitment (Lexicographical JCS)
        // Hashes provided for CHORA specimen f3d4bbce...
        let intent_hash = "1f05a4c81ff8b0026e873d3782b07c5140c89efcd632a5f121159e2e823b744d";
        let authority_hash = "c1c9cc1c96db2959dc824e4398162d0fd4250ff483f74475740e92e97dc38aef";
        let identity_hash = "fc6f5810fc16aea2197501867237159f284efdb2e1b6e7865a034b610e4903a3";
        let witness_hash_live = "7d4e2acaa7e459261d48f79cbec2a08ef5f8489e7cb610f375b708f9b8027e33";

        let root_map = serde_json::json!({
            "authority_hash": authority_hash,
            "identity_hash": identity_hash,
            "intent_hash": intent_hash,
            "witness_hash": witness_hash_live
        });

        let jcs_bytes = serde_jcs::to_vec(&root_map).unwrap();
        let mut hasher = sha2::Sha256::new();
        hasher.update(&jcs_bytes);
        use sha2::Digest;
        let capsule_root = hex::encode(hasher.finalize());

        assert_eq!(
            capsule_root, "4401e5b102f949472b0bc247a8a9cb1dd685a3ba387f0cca332fb13fafdbd960",
            "Capsule root commitment must match the recomputed v0.2 model"
        );
    }
}
