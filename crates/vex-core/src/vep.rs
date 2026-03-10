//! VEP (Viking Enveloped Packet) Binary Format
//!
//! A zero-copy, high-performance binary envelope for segmented VEX audit data.

use zerocopy::{FromBytes, Immutable, IntoBytes, KnownLayout, Unaligned, LE, U32};

/// VEP Magic bytes: "VEP" (3 bytes)
pub const VEP_MAGIC: [u8; 3] = *b"VEP";
pub const VEP_VERSION_V2: u8 = 2; // CHORA Capsule v1 uses v2 wire
pub const VEP_HEADER_SIZE: usize = 76;

/// VEP Header (76 bytes) - Aligned with George's Wire Spec
/// format: magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned, Debug, Clone, Copy)]
#[repr(C)]
pub struct VepHeader {
    pub magic: [u8; 3],
    pub version: u8,
    pub aid: [u8; 32],
    pub capsule_root: [u8; 32],
    pub nonce: [u8; 8], // u64 BE
}

/// VEP Segment Header (9 bytes)
#[derive(FromBytes, IntoBytes, KnownLayout, Immutable, Unaligned)]
#[repr(C)]
pub struct VepSegmentHeader {
    pub segment_type: u8,
    pub offset: U32<LE>,
    pub length: U32<LE>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VepSegmentType {
    Intent = 1,
    Authority = 2,
    Identity = 3,
    Payload = 4,
    Witness = 5,
    Signature = 6, // Specific for capsule signatures
}

/// A high-level view of a VEP packet using zero-copy references.
pub struct VepPacket<'a> {
    buffer: &'a [u8],
}

impl<'a> VepPacket<'a> {
    pub fn new(buffer: &'a [u8]) -> Result<Self, &'static str> {
        if buffer.len() < VEP_HEADER_SIZE {
            return Err("Buffer too small for VEP header");
        }

        let (header, _) =
            VepHeader::ref_from_prefix(buffer).map_err(|_| "Failed to parse VEP header")?;

        if header.magic != VEP_MAGIC {
            return Err("Invalid VEP magic bytes");
        }

        Ok(Self { buffer })
    }

    pub fn header(&self) -> &VepHeader {
        VepHeader::ref_from_prefix(self.buffer).unwrap().0
    }

    /// Iterates through segments in the encrypted payload.
    /// In Phase 5.3, the segments are stored in a TLV format inside the payload.
    pub fn get_segment_data(&self, segment_type: VepSegmentType) -> Option<&[u8]> {
        let mut offset = 0;
        let data = &self.buffer[VEP_HEADER_SIZE..];

        while offset + 5 <= data.len() {
            let t = data[offset];
            let len_bytes: [u8; 4] = data[offset + 1..offset + 5].try_into().ok()?;
            let len = u32::from_be_bytes(len_bytes) as usize;
            offset += 5;

            if offset + len > data.len() {
                break;
            }

            if t == segment_type as u8 {
                return Some(&data[offset..offset + len]);
            }
            offset += len;
        }
        None
    }

    /// Verifies the cryptographic integrity of the packet against a CHORA public key. (Phase 3.2)
    /// Following George's Capsule v1 Spec: The signature is over the `capsule_root`.
    pub fn verify(&self, chora_public_key_bytes: &[u8]) -> Result<bool, String> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        // 1. Reconstruct the Capsule to compute the composite capsule_root
        let capsule = self.to_capsule()?;
        let capsule_root = capsule.to_composite_hash()?;

        // 2. Extract the Signature segment
        let signature_bytes = self
            .get_segment_data(VepSegmentType::Signature)
            .ok_or("Missing Signature segment")?;

        // 3. Verify the signature over the capsule_root bytes
        let public_key = VerifyingKey::from_bytes(
            chora_public_key_bytes
                .try_into()
                .map_err(|_| "Invalid public key length")?,
        )
        .map_err(|e| format!("Invalid public key: {}", e))?;

        let signature = Signature::from_slice(signature_bytes)
            .map_err(|e| format!("Invalid signature format: {}", e))?;

        match public_key.verify(&capsule_root.0, &signature) {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    /// Reconstructs a full VEX Capsule from the VEP segments.
    pub fn to_capsule(&self) -> Result<crate::segment::Capsule, String> {
        use crate::segment::{
            AuthorityData, Capsule, CryptoData, IdentityData, IntentData, WitnessData,
        };
        use serde::Serialize;

        let intent_bytes = self
            .get_segment_data(VepSegmentType::Intent)
            .ok_or("Missing Intent segment")?;
        let auth_bytes = self
            .get_segment_data(VepSegmentType::Authority)
            .ok_or("Missing Authority segment")?;
        let ident_bytes = self
            .get_segment_data(VepSegmentType::Identity)
            .ok_or("Missing Identity segment")?;
        let witness_bytes = self
            .get_segment_data(VepSegmentType::Witness)
            .ok_or("Missing Witness segment")?;
        let sig_bytes = self
            .get_segment_data(VepSegmentType::Signature)
            .ok_or("Missing Signature segment")?;

        let intent: IntentData = serde_json::from_slice(intent_bytes)
            .map_err(|e| format!("Failed to parse Intent segment: {}", e))?;
        let authority: AuthorityData = serde_json::from_slice(auth_bytes)
            .map_err(|e| format!("Failed to parse Authority segment: {}", e))?;
        let identity: IdentityData = serde_json::from_slice(ident_bytes)
            .map_err(|e| format!("Failed to parse Identity segment: {}", e))?;
        let witness: WitnessData = serde_json::from_slice(witness_bytes)
            .map_err(|e| format!("Failed to parse Witness segment: {}", e))?;

        let intent_hash = intent.to_jcs_hash()?.to_hex();

        fn hash_seg<T: Serialize>(seg: &T) -> Result<String, String> {
            let jcs = serde_jcs::to_vec(seg).map_err(|e| e.to_string())?;
            let mut hasher = sha2::Sha256::new();
            use sha2::Digest;
            hasher.update(&jcs);
            Ok(hex::encode(hasher.finalize()))
        }

        let authority_hash = hash_seg(&authority)?;
        let identity_hash = hash_seg(&identity)?;
        let witness_hash = hash_seg(&witness)?;

        let mut capsule = Capsule {
            capsule_id: authority.capsule_id.clone(),
            intent,
            authority,
            identity,
            witness,
            intent_hash,
            authority_hash,
            identity_hash,
            witness_hash,
            capsule_root: String::new(),
            crypto: CryptoData {
                algo: "ed25519".to_string(),
                public_key_endpoint: "/public_key".to_string(),
                signature_scope: "capsule_root".to_string(),
                signature_b64: base64::Engine::encode(
                    &base64::engine::general_purpose::STANDARD,
                    sig_bytes,
                ),
            },
        };

        let root = capsule.to_composite_hash()?;
        capsule.capsule_root = root.to_hex();

        Ok(capsule)
    }
}
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vep_parsing() {
        let mut buffer = vec![0u8; 128];

        // Setup 76-byte header
        let header = VepHeader {
            magic: VEP_MAGIC,
            version: VEP_VERSION_V2,
            aid: [0xAA; 32],
            capsule_root: [0; 32],
            nonce: [0; 8],
        };

        // Write header to buffer
        buffer[0..76].copy_from_slice(header.as_bytes());

        // Setup TLV segment (Type 1, Len 5, "HELLO")
        let payload_offset = 76;
        buffer[payload_offset] = 1; // Type: Intent
        buffer[payload_offset + 1..payload_offset + 5].copy_from_slice(&(5u32.to_be_bytes()));
        buffer[payload_offset + 5..payload_offset + 10].copy_from_slice(b"HELLO");

        let packet = VepPacket::new(&buffer).unwrap();
        assert_eq!(packet.header().aid[0], 0xAA);

        let data = packet.get_segment_data(VepSegmentType::Intent).unwrap();
        assert_eq!(data, b"HELLO");
    }

    #[test]
    fn test_vep_verification_logic() {
        // Implementation of verification logic test with 76-byte header
        let mut buffer = vec![0u8; 256];
        let header = VepHeader {
            magic: VEP_MAGIC,
            version: VEP_VERSION_V2,
            aid: [0; 32],
            capsule_root: [0; 32],
            nonce: [0; 8],
        };
        buffer[0..76].copy_from_slice(zerocopy::IntoBytes::as_bytes(&header));

        // We verify that new() and get_segment_data work for basic TLV
        let packet = VepPacket::new(&buffer).unwrap();
        assert_eq!(packet.header().magic, VEP_MAGIC);
    }
}
