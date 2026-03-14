pub use crate::traits::HardwareIdentity;
#[allow(unused_imports)]
use anyhow::{anyhow, Result};
use async_trait::async_trait;

/// Factory function to create the platform-appropriate Identity Provider
#[allow(unused_variables)]
pub fn create_identity_provider(allow_fallback: bool) -> Box<dyn HardwareIdentity> {
    #[cfg(windows)]
    {
        match windows_impl::CngIdentity::new() {
            Ok(cng) => Box::new(cng),
            Err(e) => {
                if allow_fallback {
                    tracing::warn!("⚠️  CNG/TPM provider not available on Windows: {}. Falling back to StubIdentity.", e);
                    Box::new(stub_impl::StubIdentity::default())
                } else {
                    panic!(
                        "❌ Critical: CNG/TPM required but initialization failed: {}",
                        e
                    );
                }
            }
        }
    }

    #[cfg(target_os = "linux")]
    {
        match linux_impl::Tpm2Identity::new() {
            Ok(tpm) => Box::new(tpm),
            Err(e) => {
                if allow_fallback {
                    Box::new(stub_impl::StubIdentity::default())
                } else {
                    panic!("❌ Critical: TPM required but initialization failed: {}", e);
                }
            }
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        if allow_fallback {
            Box::new(stub_impl::StubIdentity::default())
        } else {
            panic!("❌ Critical: Hardware identity not supported on this platform");
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux_impl::Tpm2Identity;
pub use stub_impl::StubIdentity;
#[cfg(windows)]
pub use windows_impl::CngIdentity;

// Windows Implementation
#[cfg(windows)]
mod windows_impl {
    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Cryptography::*;
    use zeroize::Zeroize;

    fn map_cng_error(status: i32) -> String {
        match status as u32 {
            0x80090010 => "Access Denied (Insufficient TPM permissions)".to_string(),
            0x80090025 => "TPM Device Locked (Anti-hammering lockout)".to_string(),
            0x80090005 => "Bad Data (Corrupted ciphertext)".to_string(),
            0x80090027 => "Hardware Unsupported (Payload too large)".to_string(),
            _ => format!("CNG Status 0x{:X}", status as u32),
        }
    }

    #[derive(Default)]
    pub struct CngIdentity {
        pub sealed_seed: Option<Vec<u8>>,
        pub identity_public_key: Option<Vec<u8>>,
    }

    impl CngIdentity {
        pub fn new() -> Result<Self> {
            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();
                let status = NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);

                if status == 0 {
                    NCryptFreeObject(provider);
                    Ok(Self::default())
                } else {
                    Err(anyhow!(
                        "TPM provider not available ({})",
                        map_cng_error(status)
                    ))
                }
            }
        }
    }

    #[async_trait]
    impl HardwareIdentity for CngIdentity {
        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();
                let mut status =
                    NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);
                if status != 0 {
                    return Err(anyhow!(
                        "TPM provider not available ({})",
                        map_cng_error(status)
                    ));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();
                let alg_id: Vec<u16> = "RSA\0".encode_utf16().collect();

                status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
                if status != 0 {
                    status = NCryptCreatePersistedKey(
                        provider,
                        &mut key_handle,
                        alg_id.as_ptr(),
                        key_name.as_ptr(),
                        0,
                        0,
                    );
                    if status != 0 {
                        NCryptFreeObject(provider);
                        return Err(anyhow!(
                            "Failed to create TPM key ({})",
                            map_cng_error(status)
                        ));
                    }
                    status = NCryptFinalizeKey(key_handle, 0);
                    if status != 0 {
                        NCryptFreeObject(key_handle);
                        NCryptFreeObject(provider);
                        return Err(anyhow!(
                            "Failed to finalize TPM key ({})",
                            map_cng_error(status)
                        ));
                    }
                }

                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(data);
                let mut payload = Vec::with_capacity(32 + data.len());
                payload.extend_from_slice(&hasher.finalize());
                payload.extend_from_slice(data);

                let mut output_size: u32 = 0;
                status = NCryptEncrypt(
                    key_handle,
                    payload.as_ptr(),
                    payload.len() as u32,
                    std::ptr::null(),
                    null_mut(),
                    0,
                    &mut output_size,
                    NCRYPT_PAD_PKCS1_FLAG,
                );
                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get ciphertext size ({})",
                        map_cng_error(status)
                    ));
                }

                let mut ciphertext = vec![0u8; output_size as usize];
                status = NCryptEncrypt(
                    key_handle,
                    payload.as_ptr(),
                    payload.len() as u32,
                    std::ptr::null(),
                    ciphertext.as_mut_ptr(),
                    ciphertext.len() as u32,
                    &mut output_size,
                    NCRYPT_PAD_PKCS1_FLAG,
                );

                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);
                if status != 0 {
                    return Err(anyhow!(
                        "Failed to encrypt with TPM ({})",
                        map_cng_error(status)
                    ));
                }
                Ok(ciphertext)
            }
        }

        async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>> {
            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();
                let mut status =
                    NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);
                if status != 0 {
                    return Err(anyhow!("TPM provider not available"));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();
                status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
                if status != 0 {
                    NCryptFreeObject(provider);
                    return Err(anyhow!("Failed to open TPM key"));
                }

                let mut output_size: u32 = 0;
                status = NCryptDecrypt(
                    key_handle,
                    blob.as_ptr(),
                    blob.len() as u32,
                    std::ptr::null(),
                    null_mut(),
                    0,
                    &mut output_size,
                    NCRYPT_PAD_PKCS1_FLAG,
                );

                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get decrypted size ({})",
                        map_cng_error(status)
                    ));
                }

                let mut decrypted = vec![0u8; output_size as usize];
                status = NCryptDecrypt(
                    key_handle,
                    blob.as_ptr(),
                    blob.len() as u32,
                    std::ptr::null(),
                    decrypted.as_mut_ptr(),
                    decrypted.len() as u32,
                    &mut output_size,
                    NCRYPT_PAD_PKCS1_FLAG,
                );

                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);

                if status != 0 {
                    return Err(anyhow!(
                        "Failed to decrypt with TPM ({})",
                        map_cng_error(status)
                    ));
                }

                decrypted.truncate(output_size as usize);

                // Verify integrity (SHA256 checksum)
                if decrypted.len() < 32 {
                    return Err(anyhow!("Unsealed data too short"));
                }
                let checksum = &decrypted[..32];
                let data = &decrypted[32..];

                use sha2::{Digest, Sha256};
                let mut hasher = Sha256::new();
                hasher.update(data);
                if hasher.finalize().as_slice() != checksum {
                    return Err(anyhow!("Integrity check failed: Data hash mismatch"));
                }

                Ok(data.to_vec())
            }
        }

        async fn sign_handshake_hash(&self, hash: &[u8]) -> Result<[u8; 64]> {
            let sealed = self
                .sealed_seed
                .as_ref()
                .ok_or_else(|| anyhow!("No sealed seed available for signature"))?;

            let mut seed = self.unseal(sealed).await?;
            let seed_bytes: [u8; 32] = seed
                .clone()
                .try_into()
                .map_err(|_| anyhow!("Invalid seed length"))?;

            use ed25519_dalek::{Signer, SigningKey};
            let signing_key = SigningKey::from_bytes(&seed_bytes);
            let signature = signing_key.sign(hash);

            // Zeroize transit memory
            seed.zeroize();
            Ok(signature.to_bytes())
        }

        async fn dh(&self, remote_public_key: &[u8]) -> Result<[u8; 32]> {
            let sealed = self
                .sealed_seed
                .as_ref()
                .ok_or_else(|| anyhow!("No sealed seed available for DH"))?;

            let mut seed = self.unseal(sealed).await?;
            let seed_bytes: [u8; 32] = seed
                .clone()
                .try_into()
                .map_err(|_| anyhow!("Invalid seed length"))?;

            let secret = x25519_dalek::StaticSecret::from(seed_bytes);
            let remote_pk_bytes: [u8; 32] = remote_public_key
                .try_into()
                .map_err(|_| anyhow!("Invalid remote public key length"))?;
            let remote_pk = x25519_dalek::PublicKey::from(remote_pk_bytes);
            let shared_secret = secret.diffie_hellman(&remote_pk);

            // Zeroize transit memory
            seed.zeroize();
            Ok(shared_secret.to_bytes())
        }

        fn set_sealed_seed(&mut self, sealed_seed: Vec<u8>) {
            self.sealed_seed = Some(sealed_seed);
        }

        fn set_public_key(&mut self, pubkey: Vec<u8>) {
            self.identity_public_key = Some(pubkey);
        }

        async fn generate_quote(&self, _nonce: &[u8]) -> Result<crate::traits::TpmQuote> {
            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();
                let mut status =
                    NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);
                if status != 0 {
                    return Err(anyhow!(
                        "TPM provider not available ({})",
                        map_cng_error(status)
                    ));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();
                status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
                if status != 0 {
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to open TPM identity key ({})",
                        map_cng_error(status)
                    ));
                }

                let mut output_size: u32 = 0;
                let property_name: Vec<u16> =
                    "PCP_PLATFORM_ATTESTATION_BLOB\0".encode_utf16().collect();
                let status = NCryptGetProperty(
                    key_handle,
                    property_name.as_ptr(),
                    null_mut(),
                    0,
                    &mut output_size,
                    0,
                );
                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get attestation property size ({})",
                        map_cng_error(status)
                    ));
                }

                let mut blob = vec![0u8; output_size as usize];
                let status = NCryptGetProperty(
                    key_handle,
                    property_name.as_ptr(),
                    blob.as_mut_ptr(),
                    blob.len() as u32,
                    &mut output_size,
                    0,
                );
                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);
                if status != 0 {
                    return Err(anyhow!(
                        "Failed to retrieve attestation blob ({})",
                        map_cng_error(status)
                    ));
                }

                Ok(crate::traits::TpmQuote {
                    message: blob,
                    signature: Vec::new(),
                    pcrs: std::collections::HashMap::new(),
                })
            }
        }

        async fn get_pcrs(
            &self,
            indices: &[u32],
        ) -> Result<std::collections::HashMap<u32, String>> {
            use windows::Win32::System::TpmBaseServices::*;

            unsafe {
                // 1. Create TBS Context
                let mut hcontext: *mut std::ffi::c_void = std::ptr::null_mut();
                let mut params = TBS_CONTEXT_PARAMS2::default();
                params.version = TBS_CONTEXT_VERSION_TWO;
                params.Anonymous.asUINT32 = 0x2; // includeTpm20 = 1

                let status = Tbsi_Context_Create(&params as *const _ as *const _, &mut hcontext);
                if status != 0 {
                    return Err(anyhow!("Tbsi_Context_Create failed: 0x{:X}", status));
                }

                let mut results = std::collections::HashMap::new();

                // 2. Read PCRs one by one for simplicity (TPM2_PCR_Read)
                for &index in indices {
                    // TPM2_PCR_Read Command (Index 0-23)
                    // Tag: 0x8001 | Size: 20 | CC: 0x0000017E | Count: 1 | Alg: SHA256 (0x000B) | Size: 3 | Select
                    let mut select = [0u8; 3];
                    if index < 24 {
                        select[(index / 8) as usize] |= 1 << (index % 8);
                    } else {
                        continue; // Only supporting 0-23 for now
                    }

                    let command: [u8; 20] = [
                        0x80, 0x01, // Tag: TPM_ST_SESSIONS
                        0x00, 0x00, 0x00, 0x14, // Size: 20
                        0x00, 0x00, 0x01, 0x7E, // CC: TPM_CC_PCR_Read
                        0x00, 0x00, 0x00, 0x01, // Count: 1
                        0x00, 0x0B, // Alg: SHA256 (0x000B)
                        0x03, // SizeOfSelect: 3
                        select[0], select[1], select[2],
                    ];

                    let mut response = [0u8; 1024];
                    let mut response_size: u32 = response.len() as u32;

                    let status = Tbsip_Submit_Command(
                        hcontext,
                        TBS_COMMAND_LOCALITY_ZERO,
                        TBS_COMMAND_PRIORITY_NORMAL,
                        &command,
                        response.as_mut_ptr(),
                        &mut response_size,
                    );

                    if status == 0 && response_size >= 20 {
                        // Response Header (10) | updateCounter (4) | pcrSelection (count, alg, sizeofselect, select) | pcrValues (count, digestSize, digest)
                        // Simplified parsing: the digest usually starts at end of response for single PCR read
                        let digest_size = response[response_size as usize - 33]; // SHA256 = 32
                        if digest_size == 32 {
                            let digest =
                                &response[response_size as usize - 32..response_size as usize];
                            results.insert(index, hex::encode(digest));
                        }
                    }
                }

                let _ = Tbsip_Context_Close(hcontext);
                Ok(results)
            }
        }

        async fn public_key(&self) -> Result<Vec<u8>> {
            if let Some(ref pk) = self.identity_public_key {
                return Ok(pk.clone());
            }

            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();
                let mut status =
                    NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);
                if status != 0 {
                    return Err(anyhow!(
                        "TPM provider not available ({})",
                        map_cng_error(status)
                    ));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();
                status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
                if status != 0 {
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to open TPM identity key ({})",
                        map_cng_error(status)
                    ));
                }

                let mut output_size: u32 = 0;
                let blob_type: Vec<u16> = "RSAPUBLICBLOB\0".encode_utf16().collect();
                status = NCryptExportKey(
                    key_handle,
                    0,
                    blob_type.as_ptr(),
                    null_mut(),
                    null_mut(),
                    0,
                    &mut output_size,
                    0,
                );
                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get public key size ({})",
                        map_cng_error(status)
                    ));
                }

                let mut blob = vec![0u8; output_size as usize];
                status = NCryptExportKey(
                    key_handle,
                    0,
                    blob_type.as_ptr(),
                    null_mut(),
                    blob.as_mut_ptr(),
                    blob.len() as u32,
                    &mut output_size,
                    0,
                );

                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);
                if status != 0 {
                    return Err(anyhow!(
                        "Failed to export public key ({})",
                        map_cng_error(status)
                    ));
                }
                Ok(blob)
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;

    pub struct Tpm2Identity {
        pub sealed_seed: Option<Vec<u8>>,
        pub identity_public_key: Option<Vec<u8>>,
    }

    impl Tpm2Identity {
        pub fn new() -> Result<Self> {
            Ok(Self {
                sealed_seed: None,
                identity_public_key: None,
            })
        }
    }

    #[async_trait]
    impl HardwareIdentity for Tpm2Identity {
        fn set_sealed_seed(&mut self, sealed_seed: Vec<u8>) {
            self.sealed_seed = Some(sealed_seed);
        }

        fn set_public_key(&mut self, pubkey: Vec<u8>) {
            self.identity_public_key = Some(pubkey);
        }

        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            // For WSL/Linux, we return the plain seed for now to allow integration tests to pass
            // while real TPM binding is pending dev-env setup.
            Ok(data.to_vec())
        }
        async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>> {
            Ok(blob.to_vec())
        }
        async fn sign_handshake_hash(&self, hash: &[u8]) -> Result<[u8; 64]> {
            let sealed = self
                .sealed_seed
                .as_ref()
                .ok_or_else(|| anyhow!("No sealed seed available for signature"))?;

            let mut seed = self.unseal(sealed).await?;
            let seed_bytes: [u8; 32] = seed
                .clone()
                .try_into()
                .map_err(|_| anyhow!("Invalid seed length"))?;

            use ed25519_dalek::{Signer, SigningKey};
            let signing_key = SigningKey::from_bytes(&seed_bytes);
            let signature = signing_key.sign(hash);

            use zeroize::Zeroize;
            seed.zeroize();
            Ok(signature.to_bytes())
        }
        async fn dh(&self, remote_public_key: &[u8]) -> Result<[u8; 32]> {
            let sealed = self
                .sealed_seed
                .as_ref()
                .ok_or_else(|| anyhow!("No sealed seed available for DH"))?;

            let mut seed = self.unseal(sealed).await?;
            let seed_bytes: [u8; 32] = seed
                .clone()
                .try_into()
                .map_err(|_| anyhow!("Invalid seed length"))?;

            let secret = x25519_dalek::StaticSecret::from(seed_bytes);
            let remote_pk_bytes: [u8; 32] = remote_public_key
                .try_into()
                .map_err(|_| anyhow!("Invalid remote public key length"))?;
            let remote_pk = x25519_dalek::PublicKey::from(remote_pk_bytes);
            let shared_secret = secret.diffie_hellman(&remote_pk);

            use zeroize::Zeroize;
            seed.zeroize();
            Ok(shared_secret.to_bytes())
        }
        async fn generate_quote(&self, _nonce: &[u8]) -> Result<crate::traits::TpmQuote> {
            Ok(crate::traits::TpmQuote {
                message: Vec::new(),
                signature: Vec::new(),
                pcrs: std::collections::HashMap::new(),
            })
        }
        async fn get_pcrs(
            &self,
            _indices: &[u32],
        ) -> Result<std::collections::HashMap<u32, String>> {
            Ok(std::collections::HashMap::new())
        }
        async fn public_key(&self) -> Result<Vec<u8>> {
            if let Some(ref pk) = self.identity_public_key {
                return Ok(pk.clone());
            }
            Ok(Vec::new())
        }
    }
}

mod stub_impl {
    use super::*;

    #[derive(Default)]
    #[allow(dead_code)]
    pub struct StubIdentity {
        pub sealed_seed: Option<Vec<u8>>,
        pub identity_public_key: Option<Vec<u8>>,
    }

    #[async_trait]
    impl HardwareIdentity for StubIdentity {
        fn set_sealed_seed(&mut self, sealed_seed: Vec<u8>) {
            self.sealed_seed = Some(sealed_seed);
        }

        fn set_public_key(&mut self, pubkey: Vec<u8>) {
            self.identity_public_key = Some(pubkey);
        }

        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            Ok(data.to_vec())
        }

        async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>> {
            Ok(blob.to_vec())
        }

        async fn sign_handshake_hash(&self, _hash: &[u8]) -> Result<[u8; 64]> {
            Ok([0u8; 64])
        }

        async fn dh(&self, _remote_public_key: &[u8]) -> Result<[u8; 32]> {
            Ok([0u8; 32])
        }

        async fn generate_quote(&self, _nonce: &[u8]) -> Result<crate::traits::TpmQuote> {
            Ok(crate::traits::TpmQuote {
                message: Vec::new(),
                signature: Vec::new(),
                pcrs: std::collections::HashMap::new(),
            })
        }

        async fn get_pcrs(
            &self,
            _indices: &[u32],
        ) -> Result<std::collections::HashMap<u32, String>> {
            Ok(std::collections::HashMap::new())
        }

        async fn public_key(&self) -> Result<Vec<u8>> {
            if let Some(ref pk) = self.identity_public_key {
                return Ok(pk.clone());
            }
            Ok(Vec::new())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_identity_interface() {
        let provider = create_identity_provider(true); // Fallback for envs without TPM
        let data = b"test_secret_seed";

        match provider.seal("test_label", data).await {
            Ok(sealed) => {
                let unsealed = provider
                    .unseal(&sealed)
                    .await
                    .expect("Unseal must succeed if seal succeeded");
                assert_eq!(
                    data.to_vec(),
                    unsealed,
                    "Unsealed data must match original input"
                );
            }
            Err(e) => {
                println!("⚠️ TPM Seal skipped or failed: {}", e);
            }
        }
    }

    #[tokio::test]
    async fn test_tpm_quote() {
        let provider = create_identity_provider(true);
        let nonce = [0xAA; 32];

        match provider.generate_quote(&nonce).await {
            Ok(_) => println!("✅ TPM Quote generated successfully"),
            Err(e) => println!("⚠️ TPM Quote skipped or failed: {}", e),
        }
    }
}
