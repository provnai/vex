use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use anyhow::{anyhow, Context, Result};
use argon2::{password_hash::rand_core::OsRng, Argon2, Params};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;
use zeroize::Zeroize;

use crate::id::AttestAgent;

#[derive(Serialize, Deserialize)]
struct EncryptedKeyStore {
    version: u8,
    salt: String,
    nonce: String,
    ciphertext: String,
    params: ArgonParams,
    tpm_blobs: Option<TpmBlobs>,
}

#[derive(Serialize, Deserialize)]
struct TpmBlobs {
    private: String,
    public: String,
}

#[derive(Serialize, Deserialize, Clone, Copy)]
struct ArgonParams {
    m_cost: u32,
    t_cost: u32,
    p_cost: u32,
}

impl From<Params> for ArgonParams {
    fn from(p: Params) -> Self {
        Self {
            m_cost: p.m_cost(),
            t_cost: p.t_cost(),
            p_cost: p.p_cost(),
        }
    }
}

impl TryInto<Params> for ArgonParams {
    type Error = argon2::Error;
    fn try_into(self) -> Result<Params, Self::Error> {
        Params::new(
            self.m_cost,
            self.t_cost,
            self.p_cost,
            Some(Params::DEFAULT_OUTPUT_LEN),
        )
    }
}

/// KeyManager handles the secure storage and retrieval of the agent's identity.
pub struct KeyManager;

impl KeyManager {
    /// Save an agent's identity to disk, encrypted with the given password.
    pub async fn save<P: AsRef<Path>>(path: P, agent: &AttestAgent, password: &str) -> Result<()> {
        let mut salt_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut salt_bytes);
        let salt = base64::encode(salt_bytes);

        // 1. Derive Encryption Key (Argon2id) using defaults
        let params = Params::default();
        let key = Self::derive_key(password, &salt_bytes, params.clone())?;

        // 2. Encrypt the Private Seed (AES-256-GCM)
        let cipher = Aes256Gcm::new(&key);
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // We export the raw 32-byte seed
        let mut seed = agent.signing_key.to_bytes();
        let ciphertext = cipher
            .encrypt(nonce, seed.as_ref())
            .map_err(|e| anyhow!("Encryption failure: {}", e))?;
        seed.zeroize();

        // 3. Write to Disk
        let store = EncryptedKeyStore {
            version: 1,
            salt,
            nonce: base64::encode(nonce_bytes),
            ciphertext: base64::encode(ciphertext),
            params: params.into(),
            tpm_blobs: None,
        };

        let json = serde_json::to_string_pretty(&store)?;
        fs::write(path, json)?;

        Ok(())
    }

    /// Load an agent's identity from disk. If TPM blobs are present, hardware unsealing is performed.
    pub async fn load<P: AsRef<Path>>(path: P, password: &str) -> Result<AttestAgent> {
        let json = fs::read_to_string(path)?;
        let store: EncryptedKeyStore = serde_json::from_str(&json)?;

        if store.version != 1 {
            return Err(anyhow!("Unsupported keystore version: {}", store.version));
        }

        // 1. Re-derive Key using stored params
        let salt_bytes = base64::decode(&store.salt).context("Invalid salt encoding")?;
        let params: Params = store
            .params
            .try_into()
            .map_err(|e| anyhow!("Invalid Argon2 params: {}", e))?;

        let key = Self::derive_key(password, &salt_bytes, params)?;

        // 2. Decrypt Ciphertext
        let cipher = Aes256Gcm::new(&key);
        let nonce_bytes = base64::decode(&store.nonce).context("Invalid nonce encoding")?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext =
            base64::decode(&store.ciphertext).context("Invalid ciphertext encoding")?;

        let mut plaintext = cipher
            .decrypt(nonce, ciphertext.as_ref())
            .map_err(|_| anyhow!("Decryption failed: Invalid password or corrupted file"))?;

        // 3. TPM Multi-Factor (Hardware Sealing)
        let (seed, sealed_seed): ([u8; 32], Option<Vec<u8>>) = if let Some(tpm_blobs) =
            store.tpm_blobs
        {
            let provider = crate::tpm::create_identity_provider(true);
            let private = base64::decode(tpm_blobs.private).context("Invalid TPM private blob")?;
            // public blob is ignored in new trait model (CNG doesn't need it for persisted keys)

            let unsealed_data = provider.unseal(&private).await?;

            // Unsealed data is the seed
            let seed = unsealed_data
                .try_into()
                .map_err(|_| anyhow!("Invalid unsealed seed length"))?;
            (seed, Some(private))
        } else {
            let seed = plaintext
                .as_slice()
                .try_into()
                .map_err(|_| anyhow!("Invalid key length"))?;
            (seed, None)
        };

        // Zeroize plaintext after use
        plaintext.zeroize();

        let mut agent = AttestAgent::from_seed(seed);
        if let Some(ss) = sealed_seed {
            agent = agent.with_sealed_seed(ss);
        }
        Ok(agent)
    }

    /// Save an agent's identity to disk with optional TPM protection.
    pub async fn save_tpm<P: AsRef<Path>>(
        path: P,
        agent: &AttestAgent,
        password: &str,
    ) -> Result<()> {
        let mut salt_bytes = [0u8; 16];
        OsRng.fill_bytes(&mut salt_bytes);
        let salt = base64::encode(salt_bytes);

        // 1. Derive Key
        let params = Params::default();
        let key = Self::derive_key(password, &salt_bytes, params.clone())?;

        // 2. TPM Sealing
        let provider = crate::tpm::create_identity_provider(true);
        let seed = agent.signing_key.to_bytes();
        let private_blob = provider.seal("identity_seed", &seed).await?;
        let public_blob = Vec::new(); // legacy/unused in this model

        // 3. Encrypt a "dummy" or "activation" ciphertext
        // In this implementation, the actual seed is in the TPM blobs.
        // The password-derived key is required to LOAD the blobs and initialize the TPM session.
        let cipher = Aes256Gcm::new(&key);
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        // We encrypt the string "TPM_PROTECTED" to confirm password validity
        let ciphertext = cipher
            .encrypt(nonce, b"TPM_PROTECTED".as_ref())
            .map_err(|e| anyhow!("Encryption failure: {}", e))?;

        // 4. Persistence
        let store = EncryptedKeyStore {
            version: 1,
            salt,
            nonce: base64::encode(nonce_bytes),
            ciphertext: base64::encode(ciphertext),
            params: params.into(),
            tpm_blobs: Some(TpmBlobs {
                private: base64::encode(private_blob),
                public: base64::encode(public_blob),
            }),
        };

        let json = serde_json::to_string_pretty(&store)?;
        fs::write(path, json)?;

        Ok(())
    }

    fn derive_key(password: &str, salt: &[u8], params: Params) -> Result<aes_gcm::Key<Aes256Gcm>> {
        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);
        let mut output_key_material = [0u8; 32]; // AES-256 needs 32 bytes

        argon2
            .hash_password_into(password.as_bytes(), salt, &mut output_key_material)
            .map_err(|e| anyhow!("Key derivation failed: {}", e))?;

        let key = *aes_gcm::Key::<Aes256Gcm>::from_slice(&output_key_material);
        output_key_material.zeroize();
        Ok(key)
    }
}

// Helper for Base64 (since we didn't add the crate explicitly, we'll use hex for now to avoid dep bloat, or add base64)
// Let's stick to Hex for simplicity and less dependencies, aligning with attest's existing deps.
mod base64 {
    pub fn encode<T: AsRef<[u8]>>(input: T) -> String {
        hex::encode(input)
    }

    pub fn decode<T: AsRef<[u8]>>(input: T) -> Result<Vec<u8>, hex::FromHexError> {
        hex::decode(input)
    }
}
