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
}

/// Witness Data (CHORA Append-Only Log)
/// Proves the receipt issuance parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessData {
    pub chora_node_id: String,
    pub receipt_hash: String,
    pub timestamp: String,
}

/// Identity Data (Attest Pillar)
/// Proves the silicon source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityData {
    pub aid: String,
    pub identity_type: String,
}

/// Crypto verification details.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CryptoData {
    pub algo: String,
    pub public_key_endpoint: String,
    pub signature_scope: String,
    pub signature_b64: String,
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
}

impl Capsule {
    /// Compute the canonical "capsule_root" using George's Hash-of-Hashes Spec
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
        let witness_hash_hex = hash_seg(&self.witness)?;

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
        };
        let mut segment2 = segment1.clone();
        segment2.confidence = 0.9;

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_ne!(hash1, hash2, "Hashes must change when content changes");
    }
}
