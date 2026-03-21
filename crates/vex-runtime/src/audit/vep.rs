use ed25519_dalek::{Signer, SigningKey};
use serde::{Deserialize, Serialize};

use thiserror::Error;

#[derive(Error, Debug)]
pub enum VepError {
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("JCS error: {0}")]
    Jcs(String),
    #[error("Crypto error: {0}")]
    Crypto(String),
    #[error("Binary format error: {0}")]
    BinaryFormat(String),
}

/// magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
pub const VEP_HEADER_SIZE: usize = 3 + 1 + 32 + 32 + 8;
pub const VEP_MAGIC: [u8; 3] = *b"VEP";
pub const VEP_VERSION: u8 = 0x03;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntentSegment {
    pub request_sha256: String,
    pub confidence: f64,
    pub capabilities: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub magpie_source: Option<String>,
    /// New Phase 2: Plonky3 Circuit Identity
    #[serde(skip_serializing_if = "Option::is_none")]
    pub circuit_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub intent_data: Option<vex_core::segment::IntentData>,
    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: vex_core::segment::SchemaValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthoritySegment {
    pub capsule_id: String,
    pub outcome: String, // ALLOW | HALT | ESCALATE
    pub reason_code: String,
    pub trace_root: String,
    pub nonce: u64,
    /// New Phase 2 fields
    #[serde(skip_serializing_if = "Option::is_none")]
    pub escalation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub binding_status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<vex_core::ContinuationToken>,
    #[serde(skip_serializing_if = "serde_json::Value::is_null")]
    pub gate_sensors: vex_core::segment::SchemaValue,
    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: vex_core::segment::SchemaValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IdentitySegment {
    pub aid: String,
    pub identity_type: String,
    /// Platform Configuration Registers (PCRs) for hardware-rooted integrity.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pcrs: Option<std::collections::HashMap<u32, String>>,
    /// Catch-all for extra fields to preserve binary parity in JCS.
    #[serde(flatten, default)]
    pub metadata: vex_core::segment::SchemaValue,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WitnessSegment {
    pub chora_node_id: String,
    pub receipt_hash: String,
    pub timestamp: u64,
    /// Diagnostic or display-only fields that are NOT part of the commitment surface.
    #[serde(flatten, default)]
    pub metadata: vex_core::segment::SchemaValue,
}

impl WitnessSegment {
    pub fn to_commitment_hash(&self) -> Result<String, VepError> {
        // v0.3 spec: only chora_node_id and timestamp are committed.
        // receipt_hash is post-seal metadata and is excluded.
        let minimal = serde_json::json!({
            "chora_node_id": self.chora_node_id,
            "timestamp": self.timestamp,
        });

        let jcs_bytes = serde_jcs::to_vec(&minimal).map_err(|e| VepError::Jcs(e.to_string()))?;
        Ok(vex_core::merkle::Hash::digest(&jcs_bytes).to_hex())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestCommitment {
    pub canonicalization: String,
    pub payload_sha256: String,
    pub payload_encoding: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceCapsuleV0 {
    pub capsule_id: String,
    pub intent: IntentSegment,
    pub authority: AuthoritySegment,
    pub identity: IdentitySegment,
    pub witness: WitnessSegment,

    pub intent_hash: String,
    pub authority_hash: String,
    pub identity_hash: String,
    pub witness_hash: String,
    pub capsule_root: String,

    pub crypto: VepCrypto,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_commitment: Option<RequestCommitment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VepCrypto {
    pub algo: String,
    pub public_key_endpoint: String,
    pub signature_scope: String,
    pub signature_b64: String,
}

impl EvidenceCapsuleV0 {
    pub fn new(
        intent: IntentSegment,
        authority: AuthoritySegment,
        identity: IdentitySegment,
        witness: WitnessSegment,
        request_commitment: Option<RequestCommitment>,
    ) -> Result<Self, VepError> {
        let intent_hash = hash_segment(&intent)?;
        let authority_hash = hash_segment(&authority)?;
        let identity_hash = hash_segment(&identity)?;
        let witness_hash = witness.to_commitment_hash()?;

        // Root commitment: 4-leaf Merkle Tree (v0.3 spec)
        use vex_core::merkle::{Hash, MerkleTree};

        let intent_h = Hash::from_bytes(hex::decode(&intent_hash).unwrap().try_into().unwrap());
        let authority_h =
            Hash::from_bytes(hex::decode(&authority_hash).unwrap().try_into().unwrap());
        let identity_h = Hash::from_bytes(hex::decode(&identity_hash).unwrap().try_into().unwrap());
        let witness_h = Hash::from_bytes(hex::decode(&witness_hash).unwrap().try_into().unwrap());

        let leaves = vec![
            ("intent".to_string(), intent_h),
            ("authority".to_string(), authority_h),
            ("identity".to_string(), identity_h),
            ("witness".to_string(), witness_h),
        ];

        let tree = MerkleTree::from_leaves(leaves);
        let capsule_root = tree.root_hash().unwrap().to_hex();

        Ok(Self {
            capsule_id: authority.capsule_id.clone(),
            intent,
            authority,
            identity,
            witness,
            intent_hash,
            authority_hash,
            identity_hash,
            witness_hash,
            capsule_root,
            crypto: VepCrypto {
                algo: "ed25519".to_string(),
                public_key_endpoint: "https://authority.domain/v1/keys".to_string(), // Generic Placeholder
                signature_scope: "capsule_root".to_string(),
                signature_b64: String::new(),
            },
            request_commitment,
        })
    }

    pub fn sign(&mut self, signing_key: &SigningKey) -> Result<(), VepError> {
        let root_bytes = hex::decode(&self.capsule_root)
            .map_err(|e| VepError::Crypto(format!("Hex decode failed: {}", e)))?;

        let signature = signing_key.sign(&root_bytes);
        self.set_signature(signature.to_bytes().as_ref());
        Ok(())
    }

    pub fn set_signature(&mut self, signature_bytes: &[u8]) {
        use base64::Engine as _;
        self.crypto.signature_b64 =
            base64::engine::general_purpose::STANDARD.encode(signature_bytes);
    }

    pub fn to_vep_binary(&self) -> Result<Vec<u8>, VepError> {
        let mut buffer = Vec::with_capacity(1024);

        // 1. Header: magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
        buffer.extend_from_slice(&VEP_MAGIC);
        buffer.push(VEP_VERSION);

        let aid_bytes = hex::decode(&self.identity.aid)
            .map_err(|e| VepError::BinaryFormat(format!("Invalid AID hex: {}", e)))?;
        buffer.extend_from_slice(&aid_bytes);

        let root_bytes = hex::decode(&self.capsule_root)
            .map_err(|e| VepError::BinaryFormat(format!("Invalid root hex: {}", e)))?;
        buffer.extend_from_slice(&root_bytes);

        buffer.extend_from_slice(&self.authority.nonce.to_be_bytes());

        // 2. Helper to append TLV segment
        fn append_segment(buffer: &mut Vec<u8>, segment_type: u8, data: &[u8]) {
            buffer.push(segment_type);
            buffer.extend_from_slice(&(data.len() as u32).to_be_bytes());
            buffer.extend_from_slice(data);
        }

        // 3. Main Pillars (JSON serialized)
        let intent_bytes =
            serde_jcs::to_vec(&self.intent).map_err(|e| VepError::Jcs(e.to_string()))?;
        append_segment(&mut buffer, 1, &intent_bytes); // Intent

        let auth_bytes =
            serde_jcs::to_vec(&self.authority).map_err(|e| VepError::Jcs(e.to_string()))?;
        append_segment(&mut buffer, 2, &auth_bytes); // Authority

        let ident_bytes =
            serde_jcs::to_vec(&self.identity).map_err(|e| VepError::Jcs(e.to_string()))?;
        append_segment(&mut buffer, 3, &ident_bytes); // Identity

        let witness_bytes =
            serde_jcs::to_vec(&self.witness).map_err(|e| VepError::Jcs(e.to_string()))?;
        append_segment(&mut buffer, 5, &witness_bytes); // Witness

        // 4. Dedicated Magpie AST Segment (Raw Binary)
        if let Some(ast) = &self.intent.magpie_source {
            append_segment(&mut buffer, 7, ast.as_bytes());
        }

        // 5. Signature (Raw Binary)
        use base64::Engine as _;
        let sig_bytes = base64::engine::general_purpose::STANDARD
            .decode(&self.crypto.signature_b64)
            .map_err(|e| VepError::Crypto(format!("Base64 decode failed: {}", e)))?;
        append_segment(&mut buffer, 6, &sig_bytes);

        Ok(buffer)
    }
}

fn hash_segment<T: Serialize>(segment: &T) -> Result<String, VepError> {
    let jcs_bytes = serde_jcs::to_vec(segment).map_err(|e| VepError::Jcs(e.to_string()))?;
    Ok(vex_core::merkle::Hash::digest(&jcs_bytes).to_hex())
}
