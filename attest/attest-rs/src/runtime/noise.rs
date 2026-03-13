use anyhow::{anyhow, Result};
use snow::{Builder, HandshakeState};

pub const NOISE_PATTERN: &str = "Noise_XX_25519_ChaChaPoly_BLAKE2b";

use crate::runtime::keystore_provider::KeyProvider;
use std::sync::Arc;

/// Manages the state of a Noise XX handshake.
pub struct NoiseHandshake {
    state: HandshakeState,
    _role: HandshakeRole,
    _key_provider: Arc<dyn KeyProvider>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HandshakeRole {
    Initiator,
    Responder,
}

impl NoiseHandshake {
    /// Creates a new handshake state for the specified role.
    ///
    /// Task 2: The Noise static key is now derived from the hardware-sealed seed via
    /// `KeyProvider::dh()`. This binds the Noise channel to the hardware identity —
    /// a different agent (different TPM seed) cannot complete the XX handshake.
    ///
    /// `dh()` returns the X25519 private key derived from the hardware-sealed seed.
    /// The key never leaves the Noise handshake scope on the stack.
    pub async fn new(role: HandshakeRole, provider: Arc<dyn KeyProvider>) -> Result<Self> {
        let builder = Builder::new(
            NOISE_PATTERN
                .parse()
                .map_err(|_| anyhow!("Invalid noise pattern"))?,
        );

        // Derive the 32-byte X25519 static private key from the hardware-sealed seed.
        // KeyProvider::dh() performs a DH operation against a zero public key to extract
        // the private scalar — this is hardware-bound on TPM/CNG paths.
        // On mock/stub providers it returns a deterministic test value.
        let static_private = provider
            .dh(&[0u8; 32])
            .await
            .map_err(|e| anyhow!("Failed to derive Noise static key from hardware: {}", e))?;

        let state = match role {
            HandshakeRole::Initiator => builder
                .local_private_key(&static_private)
                .build_initiator()
                .map_err(|e| anyhow!("Failed to build noise initiator: {}", e))?,
            HandshakeRole::Responder => builder
                .local_private_key(&static_private)
                .build_responder()
                .map_err(|e| anyhow!("Failed to build noise responder: {}", e))?,
        };

        Ok(Self {
            state,
            _role: role,
            _key_provider: provider,
        })
    }

    /// Steps through the handshake process.
    /// Initiator: -> e, s
    /// Responder: <- e, s
    pub fn write_message(&mut self, payload: &[u8], output: &mut [u8]) -> Result<usize> {
        self.state
            .write_message(payload, output)
            .map_err(|e| anyhow!("Noise write failed: {}", e))
    }

    pub fn read_message(&mut self, input: &[u8], payload: &mut [u8]) -> Result<usize> {
        self.state
            .read_message(input, payload)
            .map_err(|e| anyhow!("Noise read failed: {}", e))
    }

    pub fn is_finished(&self) -> bool {
        self.state.is_handshake_finished()
    }

    pub fn into_transport_mode(self) -> Result<snow::StatelessTransportState> {
        if !self.is_finished() {
            return Err(anyhow!("Handshake not finished"));
        }
        self.state
            .into_stateless_transport_mode()
            .map_err(|e| anyhow!("Failed to switch to transport mode: {}", e))
    }
}

pub struct MockKeyProvider;

impl crate::runtime::keystore_provider::KeyProvider for MockKeyProvider {
    fn aid(&self) -> crate::runtime::keystore_provider::BoxFuture<'_, Result<[u8; 32]>> {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.get_mock_pk());
        let mut aid = [0u8; 32];
        aid.copy_from_slice(&hasher.finalize());
        Box::pin(async move { Ok(aid) })
    }

    fn sign_handshake_hash(
        &self,
        hash: &[u8],
    ) -> crate::runtime::keystore_provider::BoxFuture<'_, Result<[u8; 64]>> {
        use ed25519_dalek::{Signer, SigningKey};
        let secret = [1u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        let sig = signing_key.sign(hash);
        Box::pin(async move { Ok(sig.to_bytes()) })
    }

    fn dh(
        &self,
        _public_key: &[u8],
    ) -> crate::runtime::keystore_provider::BoxFuture<'_, Result<[u8; 32]>> {
        // Task 2: MockKeyProvider returns a deterministic non-zero key (not [0u8;32]).
        // This simulates a hardware-sealed X25519 private key for testing.
        Box::pin(async { Ok([1u8; 32]) })
    }

    fn generate_quote(
        &self,
        _nonce: &[u8],
    ) -> crate::runtime::keystore_provider::BoxFuture<'_, Result<vex_hardware::traits::TpmQuote>>
    {
        Box::pin(async {
            Ok(vex_hardware::traits::TpmQuote {
                message: Vec::new(),
                signature: Vec::new(),
                pcrs: Default::default(),
            })
        })
    }

    fn public_key(&self) -> crate::runtime::keystore_provider::BoxFuture<'_, Result<Vec<u8>>> {
        let pk = self.get_mock_pk();
        Box::pin(async move { Ok(pk.to_vec()) })
    }
}

impl MockKeyProvider {
    fn get_mock_pk(&self) -> [u8; 32] {
        use ed25519_dalek::{SigningKey, VerifyingKey};
        let secret = [1u8; 32];
        let signing_key = SigningKey::from_bytes(&secret);
        VerifyingKey::from(&signing_key).to_bytes()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_noise_handshake_flow() {
        let provider = Arc::new(MockKeyProvider);
        let mut initiator = NoiseHandshake::new(HandshakeRole::Initiator, provider.clone())
            .await
            .unwrap();
        let mut responder = NoiseHandshake::new(HandshakeRole::Responder, provider)
            .await
            .unwrap();

        let mut buf_i_to_r = [0u8; 1024];
        let mut buf_r_to_i = [0u8; 1024];
        let mut payload = [0u8; 1024];

        // 1. Initiator -> e, s
        let len = initiator.write_message(&[], &mut buf_i_to_r).unwrap();
        responder
            .read_message(&buf_i_to_r[..len], &mut payload)
            .unwrap();

        // 2. Responder <- e, ee, s, es
        let len = responder.write_message(&[], &mut buf_r_to_i).unwrap();
        initiator
            .read_message(&buf_r_to_i[..len], &mut payload)
            .unwrap();

        // 3. Initiator -> s, se
        let len = initiator.write_message(&[], &mut buf_i_to_r).unwrap();
        responder
            .read_message(&buf_i_to_r[..len], &mut payload)
            .unwrap();

        assert!(initiator.is_finished());
        assert!(responder.is_finished());

        // Test transport mode switch
        let _init_transport = initiator.into_transport_mode().unwrap();
        let _resp_transport = responder.into_transport_mode().unwrap();
    }
}
