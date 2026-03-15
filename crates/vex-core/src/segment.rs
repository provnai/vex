//! # VEX Segments
//!
//! Provides the data structures and JCS canonicalization for the v0.1.0 "Hardened" Commitment model.

use crate::merkle::Hash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Intent Data (VEX Pillar)
/// Proves the proposed action before execution.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IntentData {
    pub request_sha256: String,
    pub confidence: f64,
    #[serde(default)]
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magpie_source: Option<String>,

    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: serde_json::Value,
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
    #[serde(default = "default_sensor_value")]
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
    /// Compute the canonical "capsule_root" using the CHORA Hash-of-Hashes Spec
    /// `SHA256(JCS({ intent_hash, authority_hash, identity_hash, witness_hash }))`
    pub fn to_composite_hash(&self) -> Result<Hash, String> {
        // Helper to hash a single JCS serializable structure
        fn hash_seg<T: Serialize>(seg: &T) -> Result<String, String> {
            let jcs = serde_jcs::to_vec(seg).map_err(|e| e.to_string())?;
            let mut hasher = Sha256::new();
            hasher.update(&jcs);
            Ok(hex::encode(hasher.finalize()))
        }

        let intent_h = self.intent.to_jcs_hash()?;
        let intent_hash_hex = intent_h.to_hex();

        let authority_hash_hex = hash_seg(&self.authority)?;
        let identity_hash_hex = hash_seg(&self.identity)?;
        let witness_hash_hex = self.witness.to_commitment_hash()?;

        // Build the Canonical Composite Object
        let composite_root = serde_json::json!({
            "intent_hash": intent_hash_hex,
            "authority_hash": authority_hash_hex,
            "identity_hash": identity_hash_hex,
            "witness_hash": witness_hash_hex
        });

        // Hash the Composite Object
        let composite_jcs = serde_jcs::to_vec(&composite_root)
            .map_err(|e| format!("JCS Serialization of composite root failed: {}", e))?;

        let mut hasher = Sha256::new();
        hasher.update(&composite_jcs);
        let result = hasher.finalize();

        Ok(Hash::from_bytes(result.into()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_intent_segment_jcs_deterministic() {
        let segment1 = IntentData {
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
        let segment1 = IntentData {
            request_sha256: "a".into(),
            confidence: 0.5,
            capabilities: vec![],
            magpie_source: None,
            metadata: serde_json::Value::Null,
        };
        let mut segment2 = segment1.clone();
        segment2.confidence = 0.9;

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_ne!(hash1, hash2, "Hashes must change when content changes");
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

        let witness_hash = witness.to_commitment_hash().unwrap();
        assert_eq!(
            witness_hash, "af138bfd4dff7f7f28bc04617529c00db04306cd49900ab729168ce8b8a9d061",
            "Witness hash must match the CHORA live production sample"
        );

        // 2. Verify Capsule Root Commitment (Lexicographical JCS)
        // Hashes provided for CHORA specimen f3d4bbce...
        let intent_hash = "1f05a4c81ff8b0026e873d3782b07c5140c89efcd632a5f121159e2e823b744d";
        let authority_hash = "c1c9cc1c96db2959dc824e4398162d0fd4250ff483f74475740e92e97dc38aef";
        let identity_hash = "fc6f5810fc16aea2197501867237159f284efdb2e1b6e7865a034b610e4903a3";
        let witness_hash_live = "af138bfd4dff7f7f28bc04617529c00db04306cd49900ab729168ce8b8a9d061";

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
            capsule_root, "f3d4bbce71827fbe4529cc6ec6560439454dcccf3a93b603be18d7e5034f32f1",
            "Capsule root commitment must match the CHORA live production sample"
        );
    }
}
