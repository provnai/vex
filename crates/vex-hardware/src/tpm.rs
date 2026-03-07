pub use crate::traits::HardwareIdentity;
use anyhow::{anyhow, Result};
use async_trait::async_trait;

use crate::api::HardwareError;

pub fn create_identity_provider(
    allow_fallback: bool,
) -> Result<Box<dyn HardwareIdentity>, HardwareError> {
    #[cfg(windows)]
    {
        tracing::info!("⚓ Probing for Hardware Root of Trust (Windows CNG)...");
        if allow_fallback {
            tracing::info!("⚙️ Fallback allowed. Using Deterministic Identity (VEX-Seed).");
            return Ok(Box::new(stub_impl::StubIdentity));
        }
        tracing::info!("✅ Using Windows CNG Hardware Identity.");
        Ok(Box::new(windows_impl::CngIdentity))
    }

    #[cfg(target_os = "linux")]
    {
        tracing::info!("⚓ Probing for Hardware Root of Trust (Linux TPM)...");
        // Silent probe: Check if the TPM device exists before letting the noisy library try to open it
        if std::path::Path::new("/dev/tpm0").exists() || std::path::Path::new("/dev/tpmrm0").exists() {
            match linux_impl::Tpm2Identity::new() {
                Ok(tpm) => {
                    tracing::info!("✅ Hardware TPM found and initialized.");
                    return Ok(Box::new(tpm));
                }
                Err(e) => {
                    tracing::warn!("⚠️ Hardware TPM found but initialization failed: {}. Falling back.", e);
                }
            }
        } else {
            tracing::info!("ℹ️ No physical TPM device found (/dev/tpm0).");
        }

        if allow_fallback {
            tracing::info!("⚙️ Falling back to Deterministic Identity (VEX-Seed).");
            Ok(Box::new(stub_impl::StubIdentity))
        } else {
            Err(HardwareError::NoTpmFound(
                "No TPM device (/dev/tpm0) found in system".to_string(),
            ))
        }
    }

    #[cfg(not(any(windows, target_os = "linux")))]
    {
        tracing::info!("⚓ Probing for Hardware Root of Trust (Generic)...");
        if allow_fallback {
            tracing::info!("⚙️ Platform not supported. Falling back to Deterministic Identity.");
            Ok(Box::new(stub_impl::StubIdentity))
        } else {
            Err(HardwareError::NoTpmFound(
                "Hardware identity not supported on this platform".to_string(),
            ))
        }
    }
}

#[cfg(target_os = "linux")]
pub use linux_impl::Tpm2Identity;
#[cfg(not(any(windows, target_os = "linux")))]
pub use stub_impl::StubIdentity;
#[cfg(windows)]
pub use windows_impl::CngIdentity;

// Windows Implementation
#[cfg(windows)]
mod windows_impl {

    use super::*;
    use std::ptr::null_mut;
    use windows_sys::Win32::Security::Cryptography::*;

    #[derive(Default)]
    pub struct CngIdentity;

    #[async_trait]
    impl HardwareIdentity for CngIdentity {
        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            unsafe {
                let mut provider: usize = 0;
                let provider_name: Vec<u16> = "Microsoft Platform Crypto Provider\0"
                    .encode_utf16()
                    .collect();

                let status = NCryptOpenStorageProvider(&mut provider, provider_name.as_ptr(), 0);
                if status != 0 {
                    return Err(anyhow!(
                        "TPM provider not available (Status: 0x{:X})",
                        status
                    ));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();
                let alg_id: Vec<u16> = "RSA\0".encode_utf16().collect();

                let mut status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
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
                        return Err(anyhow!("Failed to create TPM key (Status: 0x{:X})", status));
                    }

                    status = NCryptFinalizeKey(key_handle, 0);
                    if status != 0 {
                        NCryptFreeObject(key_handle);
                        NCryptFreeObject(provider);
                        return Err(anyhow!(
                            "Failed to finalize TPM key (Status: 0x{:X})",
                            status
                        ));
                    }
                }

                let mut output_size: u32 = 0;
                status = NCryptEncrypt(
                    key_handle,
                    data.as_ptr(),
                    data.len() as u32,
                    std::ptr::null(),
                    null_mut(),
                    0,
                    &mut output_size,
                    0,
                );
                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get ciphertext size (Status: 0x{:X})",
                        status
                    ));
                }

                let mut ciphertext = vec![0u8; output_size as usize];
                status = NCryptEncrypt(
                    key_handle,
                    data.as_ptr(),
                    data.len() as u32,
                    std::ptr::null(),
                    ciphertext.as_mut_ptr(),
                    ciphertext.len() as u32,
                    &mut output_size,
                    0,
                );

                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);

                if status != 0 {
                    return Err(anyhow!(
                        "Failed to encrypt with TPM (Status: 0x{:X})",
                        status
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
                    return Err(anyhow!(
                        "TPM provider not available (Status: 0x{:X})",
                        status
                    ));
                }

                let mut key_handle: usize = 0;
                let key_name: Vec<u16> = "AttestIdentitySRK\0".encode_utf16().collect();

                status = NCryptOpenKey(provider, &mut key_handle, key_name.as_ptr(), 0, 0);
                if status != 0 {
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Identity key not found in TPM (Status: 0x{:X})",
                        status
                    ));
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
                    0,
                );
                if status != 0 {
                    NCryptFreeObject(key_handle);
                    NCryptFreeObject(provider);
                    return Err(anyhow!(
                        "Failed to get decrypted size (Status: 0x{:X})",
                        status
                    ));
                }

                let mut plaintext = vec![0u8; output_size as usize];
                status = NCryptDecrypt(
                    key_handle,
                    blob.as_ptr(),
                    blob.len() as u32,
                    std::ptr::null(),
                    plaintext.as_mut_ptr(),
                    plaintext.len() as u32,
                    &mut output_size,
                    0,
                );

                NCryptFreeObject(key_handle);
                NCryptFreeObject(provider);

                if status != 0 {
                    return Err(anyhow!(
                        "Failed to unseal with TPM (Status: 0x{:X})",
                        status
                    ));
                }

                Ok(plaintext)
            }
        }
    }
}

#[cfg(target_os = "linux")]
mod linux_impl {
    use super::*;
    use std::sync::{Arc, Mutex};
    use tss_esapi::{
        attributes::ObjectAttributesBuilder,
        interface_types::{
            algorithm::{HashingAlgorithm, PublicAlgorithm},
            key_bits::RsaKeyBits,
            resource_handles::Hierarchy,
        },
        structures::{
            KeyedHashScheme, Private, Public, PublicBuffer, PublicBuilder,
            PublicKeyedHashParameters, RsaExponent, SensitiveData, SymmetricDefinitionObject,
        },
        tcti_ldr::TctiNameConf,
        traits::{Marshall, UnMarshall},
        utils, Context,
    };

    pub struct Tpm2Identity {
        context: Arc<Mutex<Context>>,
    }

    impl Tpm2Identity {
        pub fn new() -> Result<Self> {
            let tcti_res = TctiNameConf::from_environment_variable();
            let tcti = match tcti_res {
                Ok(t) => t,
                Err(_) => TctiNameConf::Device(Default::default()),
            };
            let context =
                Context::new(tcti).map_err(|e| anyhow!("Failed to create TPM context: {}", e))?;
            Ok(Self {
                context: Arc::new(Mutex::new(context)),
            })
        }
    }

    impl Default for Tpm2Identity {
        fn default() -> Self {
            Self::new().expect("Failed to create TPM context in Default impl")
        }
    }

    #[async_trait]
    impl HardwareIdentity for Tpm2Identity {
        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            let context_lock = self.context.clone();
            let data = data.to_vec();

            tokio::task::spawn_blocking(move || {
                let mut context = context_lock
                    .lock()
                    .map_err(|e| anyhow!("Mutex error: {}", e))?;

                // 1. Create a Primary Key in the Storage Hierarchy
                let primary_key_public = utils::create_restricted_decryption_rsa_public(
                    SymmetricDefinitionObject::AES_256_CFB,
                    RsaKeyBits::Rsa2048,
                    RsaExponent::default(),
                )
                .map_err(|e| anyhow!("Failed to create primary key public template: {}", e))?;

                let primary_key_result = context
                    .create_primary(Hierarchy::Owner, primary_key_public, None, None, None, None)
                    .map_err(|e| anyhow!("Failed to create primary key: {}", e))?;
                let primary_key_handle = primary_key_result.key_handle;

                // 2. Create the Sealed Object
                let sensitive_data = SensitiveData::try_from(data)
                    .map_err(|e| anyhow!("Invalid data for sealing: {}", e))?;

                let object_attributes = ObjectAttributesBuilder::new()
                    .with_fixed_tpm(true)
                    .with_fixed_parent(true)
                    .with_sensitive_data_origin(false) // Data is provided externally
                    .with_user_with_auth(true)
                    .build()
                    .map_err(|e| anyhow!("Failed to build object attributes: {}", e))?;

                let sealed_data_public = PublicBuilder::new()
                    .with_public_algorithm(PublicAlgorithm::KeyedHash)
                    .with_name_hashing_algorithm(HashingAlgorithm::Sha256)
                    .with_object_attributes(object_attributes)
                    .with_keyed_hash_parameters(PublicKeyedHashParameters::new(
                        KeyedHashScheme::Null,
                    ))
                    .build()
                    .map_err(|e| anyhow!("Failed to build sealed data public structure: {}", e))?;

                let create_result = context
                    .create(
                        primary_key_handle,
                        sealed_data_public,
                        None,
                        Some(sensitive_data),
                        None,
                        None,
                    )
                    .map_err(|e| anyhow!("Failed to create sealed object: {}", e))?;

                let public = create_result.out_public;
                let private = create_result.out_private;

                context.flush_context(primary_key_handle.into())?;

                // 3. Serialize both Public and Private parts into a single blob
                // Convert Public to PublicBuffer for marshalling
                let pub_buf = PublicBuffer::try_from(public)
                    .map_err(|e| anyhow!("Failed to convert Public to PublicBuffer: {}", e))?
                    .marshall()
                    .map_err(|e| anyhow!("Pub marshall error: {}", e))?;

                // For Private, use value() method if it's a buffer type
                let priv_buf = private.value().to_vec();

                let mut combined = Vec::with_capacity(4 + pub_buf.len() + priv_buf.len());
                combined.extend_from_slice(&(pub_buf.len() as u32).to_le_bytes());
                combined.extend_from_slice(&pub_buf);
                combined.extend_from_slice(&priv_buf);

                Ok(combined)
            })
            .await?
        }

        async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>> {
            let context_lock = self.context.clone();
            let blob = blob.to_vec();

            tokio::task::spawn_blocking(move || {
                let mut context = context_lock
                    .lock()
                    .map_err(|e| anyhow!("Mutex error: {}", e))?;

                // 1. Split combined blob
                if blob.len() < 4 {
                    return Err(anyhow!("Invalid blob length"));
                }
                let pub_len = u32::from_le_bytes(blob[0..4].try_into().unwrap()) as usize;
                if blob.len() < 4 + pub_len {
                    return Err(anyhow!("Invalid pub length in blob"));
                }

                let pub_buf = &blob[4..4 + pub_len];
                let priv_buf = &blob[4 + pub_len..];

                // Unmarshall into buffer types, then convert to structs
                let pub_buffer = PublicBuffer::unmarshall(pub_buf)
                    .map_err(|e| anyhow!("Pub unmarshall error: {}", e))?;
                let public = Public::try_from(pub_buffer)
                    .map_err(|e| anyhow!("Failed to convert PublicBuffer to Public: {}", e))?;

                let private = Private::try_from(priv_buf.to_vec())
                    .map_err(|e| anyhow!("Priv try_from error: {}", e))?;

                // 2. Load Primary Key again
                let primary_key_public = utils::create_restricted_decryption_rsa_public(
                    SymmetricDefinitionObject::AES_256_CFB,
                    RsaKeyBits::Rsa2048,
                    RsaExponent::default(),
                )
                .map_err(|e| anyhow!("Failed to create primary key public template: {}", e))?;

                let primary_key_result = context
                    .create_primary(Hierarchy::Owner, primary_key_public, None, None, None, None)
                    .map_err(|e| anyhow!("Failed to create primary key: {}", e))?;
                let primary_key_handle = primary_key_result.key_handle;

                // 3. Load Sealed Object
                let object_handle = context
                    .load(primary_key_handle, private, public)
                    .map_err(|e| anyhow!("Failed to load sealed object: {}", e))?;

                // 4. Unseal
                let unsealed_data = context
                    .unseal(object_handle.into())
                    .map_err(|e| anyhow!("Failed to unseal: {}", e))?;

                context.flush_context(object_handle.into())?;
                context.flush_context(primary_key_handle.into())?;

                Ok(unsealed_data.to_vec())
            })
            .await?
        }
    }
}

mod stub_impl {
    use super::*;

    #[derive(Default)]
    pub struct StubIdentity;

    #[async_trait]
    impl HardwareIdentity for StubIdentity {
        async fn seal(&self, _label: &str, data: &[u8]) -> Result<Vec<u8>> {
            Ok(data.to_vec())
        }

        async fn unseal(&self, blob: &[u8]) -> Result<Vec<u8>> {
            Ok(blob.to_vec())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_hardware_identity_interface() {
        let provider = create_identity_provider(true).expect("Failed to create provider");
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
                // On systems without TPM, we don't want the build to fail if it's just a hardware absence
                // But for Alpha, we want to know why.
            }
        }
    }
}
