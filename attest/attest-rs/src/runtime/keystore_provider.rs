use anyhow::Result;
use std::future::Future;
use std::pin::Pin;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// Trait for cryptographic key operations required by the Noise handshake.
/// This abstraction allows us to switch between pure-software and hardware-sealed keys.
pub trait KeyProvider: Send + Sync {
    /// Returns the unique Attest Hardware ID (aid).
    fn aid(&self) -> BoxFuture<'_, Result<[u8; 32]>>;

    /// Performs a cryptographic signature on a handshake digest.
    fn sign_handshake_hash(&self, hash: &[u8]) -> BoxFuture<'_, Result<[u8; 64]>>;

    /// Performs Diffie-Hellman operations using a sealed static key.
    fn dh(&self, public_key: &[u8]) -> BoxFuture<'_, Result<[u8; 32]>>;

    /// Generates a TPM Quote over the PCR state.
    fn generate_quote(&self, nonce: &[u8])
        -> BoxFuture<'_, Result<vex_hardware::traits::TpmQuote>>;

    /// Retrieves the public identity key (AID).
    fn public_key(&self) -> BoxFuture<'_, Result<Vec<u8>>>;
}

use crate::traits::HardwareIdentity;
use vex_hardware::tpm::create_identity_provider;

/// A KeyProvider implementation that uses the hardware TPM (vex-hardware).
pub struct TpmKeyProvider {
    tpm: Box<dyn HardwareIdentity>,
}

impl TpmKeyProvider {
    /// Loads the TpmKeyProvider from the hardware, optionally with a sealed seed and public key.
    pub fn new(sealed_seed: Option<Vec<u8>>, identity_public_key: Option<Vec<u8>>) -> Result<Self> {
        let mut tpm = create_identity_provider(false);
        if let Some(ss) = sealed_seed {
            tpm.set_sealed_seed(ss);
        }
        if let Some(pk) = identity_public_key {
            tpm.set_public_key(pk);
        }
        Ok(Self { tpm })
    }
}

impl KeyProvider for TpmKeyProvider {
    fn aid(&self) -> BoxFuture<'_, Result<[u8; 32]>> {
        let tpm = &self.tpm;
        Box::pin(async move {
            let pk = tpm.public_key().await?;
            use sha2::{Digest, Sha256};
            let mut hasher = Sha256::new();
            hasher.update(&pk);
            let mut aid = [0u8; 32];
            aid.copy_from_slice(&hasher.finalize());
            Ok(aid)
        })
    }

    fn sign_handshake_hash(&self, hash: &[u8]) -> BoxFuture<'_, Result<[u8; 64]>> {
        let hash = hash.to_vec();
        Box::pin(async move { self.tpm.sign_handshake_hash(&hash).await })
    }

    fn dh(&self, remote_public_key: &[u8]) -> BoxFuture<'_, Result<[u8; 32]>> {
        let remote_public_key = remote_public_key.to_vec();
        Box::pin(async move { self.tpm.dh(&remote_public_key).await })
    }

    fn generate_quote(
        &self,
        nonce: &[u8],
    ) -> BoxFuture<'_, Result<vex_hardware::traits::TpmQuote>> {
        let nonce = nonce.to_vec();
        Box::pin(async move { self.tpm.generate_quote(&nonce).await })
    }

    fn public_key(&self) -> BoxFuture<'_, Result<Vec<u8>>> {
        Box::pin(async move { self.tpm.public_key().await })
    }
}
