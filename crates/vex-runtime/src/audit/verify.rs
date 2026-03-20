use super::vep::{
    AuthoritySegment, EvidenceCapsuleV0, IdentitySegment, IntentSegment, RequestCommitment,
    WitnessSegment,
};
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
    pub fn verify_binary(
        data: &[u8],
        public_key: Option<&[u8]>,
    ) -> Result<EvidenceCapsuleV0, VerifierError> {
        use vex_core::vep::VepPacket;

        // 1. Use Core VEP Packet for TLV extraction
        let packet = VepPacket::new(data).map_err(|e| VerifierError::Integrity(e.to_string()))?;

        // 2. Perform cryptographic verification if key provided
        if let Some(pk) = public_key {
            packet
                .verify(pk)
                .map_err(|e| VerifierError::Crypto(e.to_string()))?;
        }

        // 3. Reconstruct the High-Level Capsule (Core)
        let core_capsule = packet
            .to_capsule()
            .map_err(|e| VerifierError::Integrity(e.to_string()))?;

        // 4. Map back to Runtime VEP Capsule (V0)
        let intent = match core_capsule.intent {
            vex_core::segment::IntentData::Transparent {
                request_sha256,
                confidence,
                capabilities,
                magpie_source,
                metadata,
            } => IntentSegment {
                request_sha256,
                confidence,
                capabilities,
                magpie_source,
                circuit_id: None, // V0 verifier assumes transparent/unproven for now
                metadata,
            },
            vex_core::segment::IntentData::Shadow { .. } => {
                return Err(VerifierError::Integrity(
                    "SHADOW_INTENT_NOT_SUPPORTED_IN_V0".to_string(),
                ))
            }
        };

        let authority = AuthoritySegment {
            capsule_id: core_capsule.authority.capsule_id,
            outcome: core_capsule.authority.outcome,
            reason_code: core_capsule.authority.reason_code,
            trace_root: core_capsule.authority.trace_root,
            nonce: core_capsule.authority.nonce,
            escalation_id: core_capsule.authority.escalation_id,
            binding_status: core_capsule.authority.binding_status,
            continuation_token: core_capsule.authority.continuation_token,
            gate_sensors: core_capsule.authority.gate_sensors,
            metadata: core_capsule.authority.metadata,
        };

        let identity = IdentitySegment {
            aid: core_capsule.identity.aid,
            identity_type: core_capsule.identity.identity_type,
            pcrs: core_capsule.identity.pcrs,
            metadata: core_capsule.identity.metadata,
        };

        let witness = WitnessSegment {
            chora_node_id: core_capsule.witness.chora_node_id,
            receipt_hash: core_capsule.witness.receipt_hash,
            timestamp: core_capsule.witness.timestamp,
            metadata: core_capsule.witness.metadata,
        };

        let request_commitment = core_capsule.request_commitment.map(|rc| RequestCommitment {
            canonicalization: rc.canonicalization,
            payload_sha256: rc.payload_sha256,
            payload_encoding: rc.payload_encoding,
        });

        let mut v0 =
            EvidenceCapsuleV0::new(intent, authority, identity, witness, request_commitment)
                .map_err(|e| VerifierError::Integrity(e.to_string()))?;

        v0.crypto.signature_b64 = core_capsule.crypto.signature_b64;

        Ok(v0)
    }

    /// Re-run the formal verification on the bundled Magpie AST.
    pub async fn reverify_formal_intent(&self, capsule: &EvidenceCapsuleV0) -> Result<(), String> {
        let source = capsule
            .intent
            .magpie_source
            .as_ref()
            .ok_or("VEP_MISSING_SOURCE: No bundled Magpie AST found")?;

        let tmp_path = std::env::temp_dir().join(format!("verify_{}.mp", capsule.capsule_id));
        tokio::fs::write(&tmp_path, source)
            .await
            .map_err(|e| format!("IO_ERROR: {}", e))?;

        struct Cleanup(std::path::PathBuf);
        impl Drop for Cleanup {
            fn drop(&mut self) {
                let _ = std::fs::remove_file(&self.0);
            }
        }
        let _cleanup = Cleanup(tmp_path.clone());

        use tokio::process::Command;
        let mut cmd = Command::new(crate::utils::find_magpie_binary());
        cmd.arg("--output")
            .arg("json")
            .arg("--entry")
            .arg(&tmp_path)
            .arg("parse");

        let output = cmd
            .output()
            .await
            .map_err(|e| format!("MAGPIE_EXEC_ERROR: {}", e))?;

        if output.status.success() {
            Ok(())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            Err(format!("FORMAL_VERIFICATION_FAILED: {}", stderr))
        }
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
            magpie_source: None,
            circuit_id: None,
            metadata: serde_json::Value::Null,
        };
        let authority = AuthoritySegment {
            capsule_id: "test-capsule".to_string(),
            outcome: "ALLOW".to_string(),
            reason_code: "OK".to_string(),
            trace_root: "tr".to_string(),
            nonce: 42,
            escalation_id: None,
            binding_status: None,
            continuation_token: None,
            gate_sensors: serde_json::Value::Null,
            metadata: serde_json::Value::Null,
        };
        let identity = IdentitySegment {
            aid: hex::encode([0u8; 32]), // Mock AID
            identity_type: "mock".to_string(),
            pcrs: None,
            metadata: serde_json::Value::Null,
        };
        let witness = WitnessSegment {
            chora_node_id: "node1".to_string(),
            receipt_hash: "rh".to_string(),
            timestamp: 1710396000,
            metadata: serde_json::Value::Null,
        };

        let mut capsule =
            EvidenceCapsuleV0::new(intent, authority, identity, witness, None).unwrap();
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
            magpie_source: None,
            circuit_id: None,
            metadata: serde_json::Value::Null,
        };
        let authority = AuthoritySegment {
            capsule_id: "capsule-xyz-789".to_string(),
            outcome: "ALLOW".to_string(),
            reason_code: "VERIFIED".to_string(),
            trace_root: "tr".to_string(),
            nonce: 101,
            escalation_id: None,
            binding_status: None,
            continuation_token: None,
            gate_sensors: serde_json::Value::Null,
            metadata: serde_json::Value::Null,
        };
        let identity = IdentitySegment {
            aid: hex::encode([1u8; 32]),
            identity_type: "hardware".to_string(),
            pcrs: None,
            metadata: serde_json::Value::Null,
        };
        let witness = WitnessSegment {
            chora_node_id: "nodeB".to_string(),
            receipt_hash: "receiptB".to_string(),
            timestamp: 1710396000,
            metadata: serde_json::Value::Null,
        };

        let mut capsule =
            EvidenceCapsuleV0::new(intent, authority, identity, witness, None).unwrap();
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
