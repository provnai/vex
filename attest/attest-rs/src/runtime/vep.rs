use anyhow::{anyhow, Result};
use sha2::{Digest, Sha256};
use std::io::Write;

pub const VEP_MAGIC: &[u8; 3] = b"VEP";
pub const VEP_VERSION: u8 = 0x02;
pub const VEP_HEADER_SIZE: usize = 76;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VepHeader {
    pub aid: [u8; 32],
    pub capsule_root: [u8; 32],
    pub nonce: u64,
}

impl VepHeader {
    pub fn to_bytes(&self) -> Result<Vec<u8>> {
        let mut buffer = Vec::with_capacity(VEP_HEADER_SIZE);
        buffer.write_all(VEP_MAGIC)?;
        buffer.write_all(&[VEP_VERSION])?;
        buffer.write_all(&self.aid)?;
        buffer.write_all(&self.capsule_root)?;
        buffer.write_all(&self.nonce.to_be_bytes())?;
        Ok(buffer)
    }

    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        if bytes.len() < VEP_HEADER_SIZE {
            return Err(anyhow!("Too short"));
        }
        let mut aid = [0u8; 32];
        aid.copy_from_slice(&bytes[4..36]);
        let mut capsule_root = [0u8; 32];
        capsule_root.copy_from_slice(&bytes[36..68]);
        let nonce = u64::from_be_bytes(bytes[68..76].try_into()?);
        Ok(Self {
            aid,
            capsule_root,
            nonce,
        })
    }
}

use crate::runtime::hashing::{AuthoritySegment, SegmentHasher, SegmentType};

pub struct VepPacket {
    pub header: VepHeader,
    pub encrypted_payload: Vec<u8>,
}

pub struct VepDecrypted {
    pub intent: Option<crate::runtime::intent::Intent>,
    pub auth: Option<AuthoritySegment>,
    pub payload: Vec<u8>,
}

impl VepPacket {
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        let header = VepHeader::from_bytes(&bytes[0..VEP_HEADER_SIZE])?;
        Ok(Self {
            header,
            encrypted_payload: bytes[VEP_HEADER_SIZE..].to_vec(),
        })
    }

    pub fn encapsulate_segments(
        transport: Option<snow::StatelessTransportState>,
        header: VepHeader,
        segments: &[(SegmentType, Vec<u8>)],
    ) -> Result<Vec<u8>> {
        let mut p = header.to_bytes()?;
        let mut raw = Vec::new();

        for (seg_type, data) in segments {
            let type_byte = match seg_type {
                SegmentType::Intent => 1u8,
                SegmentType::Authority => 2u8,
                SegmentType::Identity => 3u8,
                SegmentType::Payload => 4u8,
                SegmentType::Witness => 5u8,
                SegmentType::Signature => 6u8,
            };
            raw.push(type_byte);
            raw.extend_from_slice(&(data.len() as u32).to_be_bytes());
            raw.extend_from_slice(data);
        }

        if let Some(t) = transport {
            let mut enc = vec![0u8; raw.len() + 16];
            let len = t.write_message(0, &raw, &mut enc).map_err(|e| anyhow!(e))?;
            p.extend_from_slice(&enc[..len]);
        } else {
            p.extend_from_slice(&raw);
        }
        Ok(p)
    }

    pub fn decrypt_and_verify_v2(
        &self,
        transport: &snow::StatelessTransportState,
        pk: &[u8],
    ) -> Result<VepDecrypted> {
        let mut dec = vec![0u8; self.encrypted_payload.len()];
        let len = transport
            .read_message(0, &self.encrypted_payload, &mut dec)
            .map_err(|e| anyhow!(e))?;
        dec.truncate(len);

        match self.parse_segments(&dec, pk) {
            Ok(v) => Ok(v),
            Err(e) => {
                // Fallback or detailed error
                Err(anyhow!("VEP verification failed: {}", e))
            }
        }
    }

    fn parse_segments(&self, data: &[u8], pk: &[u8]) -> Result<VepDecrypted> {
        let mut offset = 0;
        let mut intent = None;
        let mut auth: Option<AuthoritySegment> = None;
        let mut payload = Vec::new();

        while offset < data.len() {
            let type_byte = data[offset];
            let len = u32::from_be_bytes(data[offset + 1..offset + 5].try_into()?) as usize;
            offset += 5;
            let seg_data = &data[offset..offset + len];
            offset += len;

            match type_byte {
                1 => {
                    // Intent
                    let i: crate::runtime::intent::Intent = serde_json::from_slice(seg_data)?;
                    intent = Some(i);
                }
                2 => {
                    // Authority (JSON) — parse into AuthoritySegment and store
                    match serde_json::from_slice::<AuthoritySegment>(seg_data) {
                        Ok(parsed_auth) => {
                            auth = Some(parsed_auth);
                        }
                        Err(e) => {
                            // Log but don't fail — authority may use a different schema version
                            eprintln!("[vep] Warning: Authority segment parse failed: {}", e);
                        }
                    }
                }
                3 => {
                    // Identity (JSON)
                    let _: serde_json::Value = serde_json::from_slice(seg_data)?;
                }
                4 => {
                    // Payload
                    payload = seg_data.to_vec();
                }
                6 => {
                    // Signature (Hardware Signature)
                    // Verify the Ed25519 signature over the capsule_root
                    let public_key = ed25519_dalek::VerifyingKey::from_bytes(pk.try_into()?)?;
                    let signature = ed25519_dalek::Signature::from_slice(seg_data)?;
                    use ed25519_dalek::Verifier;
                    public_key.verify(&self.header.capsule_root, &signature)?;
                }
                _ => {} // Skip unknown (e.g. Witness)
            }
        }

        // Additional AID verification
        let mut h = Sha256::new();
        h.update(pk);
        let actual_aid = h.finalize();
        if actual_aid.as_slice() != self.header.aid {
            println!("AID MISMATCH!");
            println!("  Header AID: {}", hex::encode(self.header.aid));
            println!("  Actual AID: {}", hex::encode(actual_aid));
            return Err(anyhow!("AID mismatch"));
        }

        Ok(VepDecrypted {
            intent,
            auth,
            payload,
        })
    }
}

use serde_json::json;

pub struct VepBuilder;

pub struct VepBuildInput<'a, T: serde::Serialize> {
    pub nonce: u64,
    pub intent: &'a crate::runtime::intent::Intent,
    pub auth: &'a AuthoritySegment,
    pub identity: &'a T,
    pub witness: &'a crate::runtime::hashing::WitnessSegment,
    pub kp: std::sync::Arc<dyn crate::runtime::keystore_provider::KeyProvider>,
    pub transport: Option<snow::StatelessTransportState>,
    pub payload: &'a [u8],
}

impl VepBuilder {
    pub async fn build_with_hardware_quote<T: serde::Serialize>(
        input: VepBuildInput<'_, T>,
    ) -> Result<Vec<u8>> {
        let VepBuildInput {
            nonce,
            intent,
            auth,
            identity,
            witness,
            kp,
            transport,
            payload,
        } = input;

        // 1. Hash the four core pillars individually
        let h_intent = SegmentHasher::hash(intent)?;
        let h_auth = SegmentHasher::hash(auth)?;
        let h_identity = SegmentHasher::hash(identity)?;
        let h_witness = SegmentHasher::hash(witness)?;

        // 2. Build the Canonical Composite Object (George's Capsule v1 Spec)
        // Note: Aligned with vex-core/src/segment.rs line 111 (no 0x, with _hash suffix)
        let composite = json!({
            "intent_hash": hex::encode(h_intent),
            "authority_hash": hex::encode(h_auth),
            "identity_hash": hex::encode(h_identity),
            "witness_hash": hex::encode(h_witness)
        });

        // 3. capsule_root = SHA256(JCS(CompositeObject))
        let capsule_root = SegmentHasher::hash(&composite)?;

        let aid = kp.aid().await?;

        // 4. Generate Hardware Attestations
        // A. Ed25519 Signature (Identity proof over capsule_root - Required for vex-core verify())
        let sig_bytes = kp.sign_handshake_hash(&capsule_root).await?;

        // 5. Build the final segment array using TLV indices
        let segments = vec![
            (SegmentType::Intent, serde_json::to_vec(intent)?),
            (SegmentType::Authority, serde_json::to_vec(auth)?),
            (SegmentType::Identity, serde_json::to_vec(identity)?), // Must be IdentityData for vex-core to_capsule()
            (SegmentType::Witness, serde_json::to_vec(witness)?),
            (SegmentType::Payload, payload.to_vec()),
            (SegmentType::Signature, sig_bytes.to_vec()), // Type 6: Ed25519 signature
        ];

        VepPacket::encapsulate_segments(
            transport,
            VepHeader {
                aid,
                capsule_root,
                nonce,
            },
            &segments,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::VepBuildInput;
    use super::*;
    use crate::runtime::keystore_provider::KeyProvider;
    use crate::runtime::noise::MockKeyProvider;
    use serde_json::json;
    use snow::Builder;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_vep_hardware_binding_roundtrip() {
        let pattern: snow::params::NoiseParams =
            "Noise_XX_25519_ChaChaPoly_BLAKE2b".parse().unwrap();
        let i_static = [1u8; 32];
        let r_static = [2u8; 32];
        let mut i = Builder::new(pattern.clone())
            .local_private_key(&i_static)
            .build_initiator()
            .unwrap();
        let mut r = Builder::new(pattern.clone())
            .local_private_key(&r_static)
            .build_responder()
            .unwrap();
        let mut b = [0u8; 1024];
        let mut p = [0u8; 1024];
        let len = i.write_message(&[], &mut b).unwrap();
        r.read_message(&b[..len], &mut p).unwrap();
        let len = r.write_message(&[], &mut b).unwrap();
        i.read_message(&b[..len], &mut p).unwrap();
        let len = i.write_message(&[], &mut b).unwrap();
        r.read_message(&b[..len], &mut p).unwrap();

        let t_i = i.into_stateless_transport_mode().unwrap();
        let t_r = r.into_stateless_transport_mode().unwrap();

        let kp = Arc::new(MockKeyProvider);
        let intent = crate::runtime::intent::Intent::new("test".into(), "test goal".into());

        let witness = crate::runtime::hashing::WitnessSegment {
            chora_node_id: "test-node".into(),
            receipt_hash: "abcd1234abcd".into(),
            timestamp: "2023-03-15T14:01:28Z".into(), // RFC3339 String — matches hashing.rs:41
        };

        let bytes = VepBuilder::build_with_hardware_quote(VepBuildInput {
            nonce: 123,
            intent: &intent,
            auth: &AuthoritySegment {
                capsule_id: "test-capsule-id".into(), // required field
                outcome: "ALLOW".into(),              // required field
                reason_code: "TEST_OK".into(),        // required field
                trace_root: [0u8; 32],
                nonce: 123,
            },
            identity: &json!({"i":1}),
            witness: &witness,
            kp: kp.clone(),
            transport: Some(t_i),
            payload: b"secret",
        })
        .await
        .unwrap();

        let packet = VepPacket::from_bytes(&bytes).unwrap();
        let pk = kp.public_key().await.unwrap();
        let dec = packet.decrypt_and_verify_v2(&t_r, &pk).unwrap();
        assert_eq!(dec.payload, b"secret");
        assert_eq!(dec.intent.unwrap().goal, "test goal");
    }
}
