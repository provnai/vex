//! # VEX Segments
//!
//! Provides the data structures and JCS canonicalization for the v0.2.0 "Segmented Commitment" model.

use crate::merkle::Hash;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

/// Intent Data (VEX Pillar)
/// Proves the proposed action before encryption.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct IntentData {
    pub id: String,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "ticketId", skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constraints: Vec<String>,
    #[serde(
        rename = "acceptanceCriteria",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub acceptance_criteria: Vec<String>,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "closedAt", skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
}

// IntentSegment removed - IntentData used directly

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
    pub nonce: u64,
    pub trace_root: [u8; 32],
}

/// Witness Data (CHORA Append-Only Log)
/// Proves the receipt issuance parameters.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct WitnessData {
    pub chora_node_id: String,
    pub receipt_hash: String,
    pub timestamp: u64,
}

/// Identity Data (Attest Pillar)
/// Proves the silicon source.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct IdentityData {
    pub agent: String,
    pub tpm: String,
}

/// A Composite Evidence Capsule (The v0.2.0 "Zero-Trust Singularity" Root)
/// Binds Intent, Authority, Identity, and Witness into a single commitment.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Capsule {
    /// VEX Pillar: What was intended
    pub intent: IntentData,
    /// CHORA Pillar: Who authorized it
    pub authority: AuthorityData,
    /// ATTEST Pillar: Where it executed (Silicon)
    pub identity: IdentityData,
    /// CHORA Log Pillar: Where the receipt lives
    pub witness: WitnessData,
    /// Ed25519 signature from the CHORA witness node
    pub chora_signature: String,
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

        let intent_hash = self.intent.to_jcs_hash()?; // returns Hash, need hex
        let intent_hash_hex = intent_hash.to_hex();

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
        let id = "agent-1".to_string();
        let goal = "test goal".to_string();
        let status = "OPEN".to_string();

        let segment1 = IntentData {
            id: id.clone(),
            goal: goal.clone(),
            description: None,
            ticket_id: None,
            constraints: vec![],
            acceptance_criteria: vec![],
            status: status.clone(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            closed_at: None,
        };
        let segment2 = segment1.clone();

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_eq!(hash1, hash2, "JCS hashing must be deterministic");
    }

    #[test]
    fn test_intent_segment_content_change() {
        let segment1 = IntentData {
            id: "a".into(),
            goal: "g".into(),
            description: None,
            ticket_id: None,
            constraints: vec![],
            acceptance_criteria: vec![],
            status: "s".into(),
            created_at: "t".into(),
            closed_at: None,
        };
        let mut segment2 = segment1.clone();
        segment2.goal = "different".into();

        let hash1 = segment1.to_jcs_hash().unwrap();
        let hash2 = segment2.to_jcs_hash().unwrap();

        assert_ne!(hash1, hash2, "Hashes must change when content changes");
    }
}
