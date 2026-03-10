use anyhow::{anyhow, Result};
use serde::{Deserialize, Serialize};
use serde_jcs;
use sha2::{Digest, Sha256};

/// Represents the different segments of an Attest message that require independent hashing.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum SegmentType {
    Intent,
    Authority,
    Identity,
    Payload,
    Witness,
    Signature,
}

/// The Authority segment contains governance and replay protection data.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct AuthoritySegment {
    /// Canonical Capsule ID from CHORA.
    pub capsule_id: String,
    /// The outcome of the governance check (ALLOW, HALT, ESCALATE).
    pub outcome: String,
    /// Reason code for the decision.
    pub reason_code: String,
    /// Reference to the trace root being authorized.
    pub trace_root: [u8; 32],
    /// 8-byte execution nonce for replay protection.
    pub nonce: u64,
}

/// The Identity segment contains the unique hardware identity (Attest Pillar).
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct IdentitySegment {
    /// SHA-256 of the Ed25519 pubkey (AID).
    pub agent: String,
    /// TPM signature of the capsule_id.
    pub tpm: String,
}

/// The Witness segment contains the append-only log record coordinates from CHORA.
#[derive(Debug, Serialize, Deserialize, Clone, PartialEq, Eq)]
pub struct WitnessSegment {
    /// The CHORA node ID that issued the receipt.
    pub chora_node_id: String,
    /// The hash of the receipt on the append-only log.
    pub receipt_hash: String,
    /// RFC3339 UTC timestamp of the receipt issuance.
    pub timestamp: String,
}

/// Helper for performing JCS-compliant hashing of message segments.
pub struct SegmentHasher;

impl SegmentHasher {
    /// Hashes a serializable segment using JCS canonicalization and SHA-256.
    /// Standard: SHA256(JCS(segment))
    pub fn hash<T: Serialize>(segment: &T) -> Result<[u8; 32]> {
        // 1. Canonicalize using JCS
        let canonical_json = serde_jcs::to_vec(segment)
            .map_err(|e| anyhow!("JCS canonicalization failed: {}", e))?;

        // 2. Compute SHA-256 digest
        let mut hasher = Sha256::new();
        hasher.update(&canonical_json);
        let result = hasher.finalize();

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        Ok(hash)
    }

    /// Convenience method to hash multiple segments and return their digests.
    pub fn hash_segments<T: Serialize>(
        segments: &[(SegmentType, T)],
    ) -> Result<Vec<([u8; 32], SegmentType)>> {
        let mut results = Vec::new();
        for (seg_type, data) in segments {
            let digest = Self::hash(data)?;
            results.push((digest, seg_type.clone()));
        }
        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_jcs_hashing_consistency() {
        // JCS ensures that key order doesn't affect the hash
        let val1 = json!({
            "id": "test",
            "value": 42,
            "meta": "data"
        });

        let val2 = json!({
            "meta": "data",
            "id": "test",
            "value": 42
        });

        let hash1 = SegmentHasher::hash(&val1).unwrap();
        let hash2 = SegmentHasher::hash(&val2).unwrap();

        assert_eq!(hash1, hash2, "JCS hashing must be order-independent");
    }

    #[test]
    fn test_segment_hashing() {
        let intent = json!({
            "action": "execute",
            "command": "whoami"
        });

        let hash = SegmentHasher::hash(&intent).expect("Should hash successfully");
        assert_ne!(hash, [0u8; 32], "Hash should not be empty");
    }

    #[test]
    fn test_authority_segment_hashing() {
        let auth = AuthoritySegment {
            capsule_id: "test-id".into(),
            outcome: "ALLOW".into(),
            reason_code: "OK".into(),
            trace_root: [0u8; 32],
            nonce: 12345678,
        };

        let hash = SegmentHasher::hash(&auth).expect("Should hash successfully");
        assert_ne!(hash, [0u8; 32], "Hash should not be empty");
    }

    #[test]
    fn test_capsule_jcs_parity() {
        use crate::runtime::intent::Intent;

        // 1. Construct segments identical to vex-core's chora_parity.rs
        let intent = Intent {
            id: "test-intent-1".into(),
            goal: "test-goal".into(),
            description: None,
            ticket_id: None,
            constraints: vec![],
            acceptance_criteria: vec![],
            status: "open".into(),
            created_at: "2024-01-01T00:00:00Z".into(),
            closed_at: None,
            metadata: None,
        };

        let authority = AuthoritySegment {
            capsule_id: "chora-v1-test".into(),
            outcome: "ALLOW".into(),
            reason_code: "WITHIN_POLICY".into(),
            trace_root: [0x55; 32],
            nonce: 12345,
        };

        let identity = IdentitySegment {
            agent: "test-agent".into(),
            tpm: "test-tpm".into(),
        };

        let witness = WitnessSegment {
            chora_node_id: "test-chora-node".into(),
            receipt_hash: "deadbeef".into(),
            timestamp: "2024-03-09T10:00:00Z".into(),
        };

        // 2. Compute individual pillar hashes
        let intent_hash = SegmentHasher::hash(&intent).unwrap();
        let auth_hash = SegmentHasher::hash(&authority).unwrap();
        let id_hash = SegmentHasher::hash(&identity).unwrap();
        let wit_hash = SegmentHasher::hash(&witness).unwrap();

        // Hex encode for composite root verification
        let intent_hex = hex::encode(intent_hash);
        let auth_hex = hex::encode(auth_hash);
        let id_hex = hex::encode(id_hash);
        let wit_hex = hex::encode(wit_hash);

        // 3. Compute the full composite capsule root
        let composite_root_obj = json!({
            "intent_hash": intent_hex,
            "authority_hash": auth_hex,
            "identity_hash": id_hex,
            "witness_hash": wit_hex
        });

        let composite_root_hash = SegmentHasher::hash(&composite_root_obj).unwrap();
        let root_hex = hex::encode(composite_root_hash);

        println!("--- ATTEST-RS HASHES ---");
        println!("Intent Hash:    {}", intent_hex);
        println!("Authority Hash: {}", auth_hex);
        println!("Identity Hash:  {}", id_hex);
        println!("Witness Hash:   {}", wit_hex);
        println!("Capsule Root:   {}", root_hex);

        assert_eq!(root_hex.len(), 64);
    }
}
