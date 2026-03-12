use super::vep::{EvidenceCapsuleV0, VEP_HEADER_SIZE, VEP_MAGIC, VEP_VERSION};
use base64::Engine as _;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum VerifierError {
    #[error("Invalid magic: expected VEP, got {0:?}")]
    InvalidMagic([u8; 3]),
    #[error("Unsupported version: expected {0}, got {1}")]
    UnsupportedVersion(u8, u8),
    #[error("Header size mismatch")]
    HeaderTooSmall,
    #[error("JSON parsing error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Integrity error: {0}")]
    Integrity(String),
    #[error("Cryptographic error: {0}")]
    Crypto(String),
}

/// The VepVerifier performs independent validation of Verifiable Evidence Packets.
pub struct VepVerifier;

impl VepVerifier {
    /// Verify a raw VEP binary blob.
    ///
    /// Sequence:
    /// 1. Parse Header (Magic, Version, AID, Root, Nonce)
    /// 2. Hash JSON payload and compare with header Root
    /// 3. Re-compute Merkle segments to verify internal consistency
    /// 4. (Optional) Verify Ed25519 signature if public key provided
    pub fn verify_binary(
        data: &[u8],
        public_key: Option<&[u8]>,
    ) -> Result<EvidenceCapsuleV0, VerifierError> {
        if data.len() < VEP_HEADER_SIZE {
            return Err(VerifierError::HeaderTooSmall);
        }

        // 1. Check Magic
        let mut magic = [0u8; 3];
        magic.copy_from_slice(&data[0..3]);
        if magic != VEP_MAGIC {
            return Err(VerifierError::InvalidMagic(magic));
        }

        // 2. Check Version
        let version = data[3];
        if version != VEP_VERSION {
            return Err(VerifierError::UnsupportedVersion(VEP_VERSION, version));
        }

        // 3. Extract Header Fields for verification
        let header_aid = hex::encode(&data[4..36]);
        let header_root = hex::encode(&data[36..68]);
        let header_nonce = u64::from_be_bytes(data[68..76].try_into().unwrap());

        // 4. Parse JSON Payload
        let json_payload = &data[VEP_HEADER_SIZE..];
        let capsule: EvidenceCapsuleV0 = serde_json::from_slice(json_payload)?;

        // 5. Cross-Verification (Header vs JSON)
        if capsule.identity.aid != header_aid {
            return Err(VerifierError::Integrity(format!(
                "AID mismatch: header {} != json {}",
                header_aid, capsule.identity.aid
            )));
        }

        if capsule.capsule_root != header_root {
            return Err(VerifierError::Integrity(format!(
                "Root mismatch: header {} != json {}",
                header_root, capsule.capsule_root
            )));
        }

        if capsule.authority.nonce != header_nonce {
            return Err(VerifierError::Integrity(format!(
                "Nonce mismatch: header {} != json {}",
                header_nonce, capsule.authority.nonce
            )));
        }

        // 6. Cryptographic Integrity: Re-compute everything
        let recomputed = EvidenceCapsuleV0::new(
            capsule.intent.clone(),
            capsule.authority.clone(),
            capsule.identity.clone(),
            capsule.witness.clone(),
        )
        .map_err(|e| VerifierError::Integrity(e.to_string()))?;

        if recomputed.capsule_root != capsule.capsule_root {
            return Err(VerifierError::Integrity(format!(
                "Merkle Root corruption: claimed {} != computed {}",
                capsule.capsule_root, recomputed.capsule_root
            )));
        }

        // 7. Signature Verification (if key available)
        if let Some(pk_bytes) = public_key {
            let verifier = VerifyingKey::from_bytes(
                pk_bytes
                    .try_into()
                    .map_err(|_| VerifierError::Crypto("Invalid public key length".to_string()))?,
            )
            .map_err(|e| VerifierError::Crypto(e.to_string()))?;

            let sig_bytes = base64::engine::general_purpose::STANDARD
                .decode(&capsule.crypto.signature_b64)
                .map_err(|e| VerifierError::Crypto(format!("Base64 decode failed: {}", e)))?;

            let signature = Signature::from_bytes(
                sig_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| VerifierError::Crypto("Invalid signature length".to_string()))?,
            );

            let root_bytes = hex::decode(&capsule.capsule_root)
                .map_err(|e| VerifierError::Crypto(format!("Root hex decode failure: {}", e)))?;

            verifier.verify(&root_bytes, &signature).map_err(|e| {
                VerifierError::Crypto(format!("Signature validation failed: {}", e))
            })?;
        }

        Ok(capsule)
    }
}

#[cfg(test)]
mod tests {
    use super::super::vep::{AuthoritySegment, IdentitySegment, IntentSegment, WitnessSegment};
    use super::*;
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_vep_end_to_end_verification() {
        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);
        let verifying_key = signing_key.verifying_key();

        let intent = IntentSegment {
            request_sha256: "aabbcc".to_string(),
            confidence: 0.9,
            capabilities: vec!["test".to_string()],
        };
        let authority = AuthoritySegment {
            capsule_id: "test-capsule".to_string(),
            outcome: "ALLOW".to_string(),
            reason_code: "OK".to_string(),
            trace_root: "tr".to_string(),
            nonce: 42,
        };
        let identity = IdentitySegment {
            aid: hex::encode([0u8; 32]), // Mock AID
            identity_type: "mock".to_string(),
        };
        let witness = WitnessSegment {
            chora_node_id: "node1".to_string(),
            receipt_hash: "rh".to_string(),
            timestamp: "now".to_string(),
        };

        let mut capsule = EvidenceCapsuleV0::new(intent, authority, identity, witness).unwrap();
        capsule.sign(&signing_key).unwrap();

        let binary = capsule.to_vep_binary().unwrap();

        // Verify with correct key
        let verified = VepVerifier::verify_binary(&binary, Some(verifying_key.as_bytes())).unwrap();
        assert_eq!(verified.capsule_id, "test-capsule");

        // Verify with no key (Integrity only)
        let integrity_only = VepVerifier::verify_binary(&binary, None).unwrap();
        assert_eq!(integrity_only.capsule_id, "test-capsule");

        // Tamper test (change a byte in the header root)
        let mut tampered = binary.clone();
        tampered[40] ^= 0xFF;
        let err = VepVerifier::verify_binary(&tampered, None).unwrap_err();
        assert!(matches!(err, VerifierError::Integrity(_)));
    }

    #[tokio::test]
    async fn test_audit_store_vep_integration() {
        use crate::audit::vep::{AuthoritySegment, IdentitySegment, IntentSegment, WitnessSegment};
        use std::sync::Arc;
        use vex_core::audit::AuditEventType;
        use vex_persist::backend::MemoryBackend;
        use vex_persist::AuditStore;

        let backend = Arc::new(MemoryBackend::new());
        let store = AuditStore::new(backend);
        let tenant = "handshake-test";

        let mut csprng = OsRng;
        let signing_key = SigningKey::generate(&mut csprng);

        // 1. Create a VEP
        let intent = IntentSegment {
            request_sha256: "deadbeef".to_string(),
            confidence: 1.0,
            capabilities: vec!["audit".to_string()],
        };
        let authority = AuthoritySegment {
            capsule_id: "capsule-xyz-789".to_string(),
            outcome: "ALLOW".to_string(),
            reason_code: "VERIFIED".to_string(),
            trace_root: "tr".to_string(),
            nonce: 101,
        };
        let identity = IdentitySegment {
            aid: hex::encode([1u8; 32]),
            identity_type: "hardware".to_string(),
        };
        let witness = WitnessSegment {
            chora_node_id: "nodeB".to_string(),
            receipt_hash: "receiptB".to_string(),
            timestamp: "2024-03-12T00:00:00Z".to_string(),
        };

        let mut capsule = EvidenceCapsuleV0::new(intent, authority, identity, witness).unwrap();
        capsule.sign(&signing_key).unwrap();
        let binary = capsule.to_vep_binary().unwrap();

        // 2. Log to AuditStore
        store
            .log(
                tenant,
                AuditEventType::GateDecision,
                vex_core::audit::ActorType::System("verifier".to_string()),
                None,
                serde_json::json!({
                    "authority": {
                        "capsule_id": "capsule-xyz-789",
                        "outcome": "ALLOW",
                        "reason_code": "VERIFIED",
                        "nonce": 101
                    }
                }),
                None,
                Some("receiptB".to_string()),
                Some(binary.clone()),
            )
            .await
            .unwrap();

        // 3. Auditor Handshake: Retrieve by Capsule ID
        let retrieved_blob = store
            .get_vep_by_capsule_id(tenant, "capsule-xyz-789")
            .await
            .unwrap();
        assert!(retrieved_blob.is_some());
        let blob = retrieved_blob.unwrap();
        assert_eq!(blob, binary);

        // 4. Auditor Handshake: Cryptographic Verification
        let result =
            VepVerifier::verify_binary(&blob, Some(signing_key.verifying_key().as_bytes()))
                .unwrap();
        assert_eq!(result.capsule_id, "capsule-xyz-789");
        assert_eq!(result.authority.nonce, 101);
    }
}
