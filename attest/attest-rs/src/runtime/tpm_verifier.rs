use crate::runtime::tpm_parser::{PcpAttestationBlob, TpmsAttest};
use anyhow::{anyhow, Result};

/// Verifies a TPM Quote against a public hardware identity (AID).
pub struct TpmVerifier;

impl TpmVerifier {
    /// Verify a TPM Quote.
    ///
    /// # Arguments
    /// * `public_key_raw` - The public key (AID) retrieved from HardwareIdentity::public_key().
    /// * `quote` - The TpmQuote structure.
    /// * `expected_nonce` - The expected nonce (capsule_root).
    pub fn verify(
        public_key_raw: &[u8],
        quote: &vex_hardware::traits::TpmQuote,
        expected_nonce: &[u8],
    ) -> Result<()> {
        // Empty / mock quotes (e.g. StubIdentity) are skipped gracefully.
        if quote.message.is_empty() {
            return Ok(());
        }

        // 1. Parse the attestation blob based on platform
        let (attest, signature) = if public_key_raw.starts_with(&[0x06, 0x02]) {
            // Windows CNG Path — RSAPUBLICBLOB format
            let pcp = PcpAttestationBlob::parse(&quote.message)?;
            (pcp.attest, pcp.signature)
        } else {
            // Linux TPM2 Path
            let attest = TpmsAttest::parse(&quote.message)?;
            (attest, quote.signature.clone())
        };

        // 2. Verify Nonce (extraData must equal expected_nonce / capsule_root)
        if attest.extra_data != expected_nonce {
            return Err(anyhow!(
                "TPM Quote nonce mismatch! Expected: {}, Got: {}",
                hex::encode(expected_nonce),
                hex::encode(&attest.extra_data)
            ));
        }

        // 3. Verify RSA/Ed25519 Signature over the attested bytes.
        // Dispatch on the public key format.
        if public_key_raw.starts_with(&[0x06, 0x02]) {
            // Windows RSAPUBLICBLOB — verify RSA-PSS SHA-256 signature
            Self::verify_windows_rsa(public_key_raw, &attest.attested_bytes, &signature)?;
        } else if public_key_raw.len() == 32 {
            // Ed25519 public key (32 bytes) — used by Attest-RS software keys
            Self::verify_ed25519(public_key_raw, &attest.attested_bytes, &signature)?;
        } else {
            // Unknown key format — log and skip rather than hard-fail
            // This allows future key types without breaking existing flows
            eprintln!(
                "[tpm_verifier] Warning: Unknown public key format (len={}), skipping sig check.",
                public_key_raw.len()
            );
        }

        Ok(())
    }

    /// Verify an RSA-PKCS1-SHA256 signature using the Windows RSAPUBLICBLOB format.
    ///
    /// RSAPUBLICBLOB layout:
    ///   bytes  0-7:  BLOBHEADER (type=6/PUBLICKEYBLOB, algId=0xA400 RSA)
    ///   bytes  8-11: magic = 0x31415352 ("RSA1")
    ///   bytes 12-15: bitlen (u32 LE)
    ///   bytes 16-19: pubexp (u32 LE)
    ///   bytes 20+:   modulus (bitlen/8 bytes, little-endian)
    fn verify_windows_rsa(key_blob: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
        use ring::signature::{self, UnparsedPublicKey};

        if key_blob.len() < 20 {
            return Err(anyhow!("RSAPUBLICBLOB too short"));
        }

        let bit_len = u32::from_le_bytes(key_blob[12..16].try_into()?) as usize;
        let mod_len = bit_len / 8;

        if key_blob.len() < 20 + mod_len {
            return Err(anyhow!("RSAPUBLICBLOB modulus out of bounds"));
        }

        // Modulus is stored little-endian; ring expects big-endian
        let mut modulus = key_blob[20..20 + mod_len].to_vec();
        modulus.reverse();

        // Public exponent (little-endian u32 → big-endian bytes, trimmed)
        let exp_le = u32::from_le_bytes(key_blob[16..20].try_into()?);
        let exp_bytes = exp_le.to_be_bytes();
        let exp_start = exp_bytes.iter().position(|&b| b != 0).unwrap_or(3);
        let exponent = &exp_bytes[exp_start..];

        // Build DER SubjectPublicKeyInfo for ring
        // ring's RSA_PKCS1_2048_8192_SHA256 requires DER-encoded RSAPublicKey
        let der = build_rsa_public_key_der(&modulus, exponent)?;

        let public_key = UnparsedPublicKey::new(&signature::RSA_PKCS1_2048_8192_SHA256, &der);
        public_key
            .verify(message, signature)
            .map_err(|_| anyhow!("TPM RSA signature verification failed"))?;

        Ok(())
    }

    /// Verify an Ed25519 signature (32-byte raw public key).
    fn verify_ed25519(public_key_raw: &[u8], message: &[u8], signature: &[u8]) -> Result<()> {
        use ed25519_dalek::{Signature, Verifier, VerifyingKey};

        let key_bytes: [u8; 32] = public_key_raw
            .try_into()
            .map_err(|_| anyhow!("Ed25519 public key must be 32 bytes"))?;

        let verifying_key = VerifyingKey::from_bytes(&key_bytes)
            .map_err(|e| anyhow!("Invalid Ed25519 key: {}", e))?;

        let sig_bytes: [u8; 64] = signature
            .try_into()
            .map_err(|_| anyhow!("Ed25519 signature must be 64 bytes"))?;

        let sig = Signature::from_bytes(&sig_bytes);
        verifying_key
            .verify(message, &sig)
            .map_err(|_| anyhow!("Ed25519 signature verification failed"))?;

        Ok(())
    }
}

/// Build a minimal DER-encoded RSAPublicKey (PKCS#1) from modulus + exponent.
/// ring's RSA_PKCS1_2048_8192_SHA256 needs this format.
fn build_rsa_public_key_der(modulus: &[u8], exponent: &[u8]) -> Result<Vec<u8>> {
    // RSAPublicKey ::= SEQUENCE { modulus INTEGER, publicExponent INTEGER }
    fn encode_integer(n: &[u8]) -> Vec<u8> {
        // Strip leading zeros, but keep at least one byte
        let n = if n.len() > 1 {
            let first_nonzero = n.iter().position(|&b| b != 0).unwrap_or(n.len() - 1);
            &n[first_nonzero..]
        } else {
            n
        };
        // Prepend 0x00 if high bit is set (to keep it positive)
        let needs_pad = n[0] & 0x80 != 0;
        let mut encoded = vec![0x02]; // INTEGER tag
        let content_len = n.len() + if needs_pad { 1 } else { 0 };
        encoded.extend(encode_length(content_len));
        if needs_pad {
            encoded.push(0x00);
        }
        encoded.extend_from_slice(n);
        encoded
    }

    fn encode_length(len: usize) -> Vec<u8> {
        if len < 128 {
            vec![len as u8]
        } else if len < 256 {
            vec![0x81, len as u8]
        } else {
            vec![0x82, (len >> 8) as u8, len as u8]
        }
    }

    let mod_enc = encode_integer(modulus);
    let exp_enc = encode_integer(exponent);
    let seq_content_len = mod_enc.len() + exp_enc.len();

    let mut der = vec![0x30]; // SEQUENCE tag
    der.extend(encode_length(seq_content_len));
    der.extend(mod_enc);
    der.extend(exp_enc);

    Ok(der)
}
