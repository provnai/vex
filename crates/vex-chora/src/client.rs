use async_trait::async_trait;

/// Trait for external authority clients.
/// This ensures VEX remains neutral and can support multiple witness providers.
#[async_trait]
pub trait AuthorityClient {
    async fn request_attestation(&self, payload: &[u8]) -> Result<Vec<u8>, String>;
    async fn verify_witness_signature(
        &self,
        payload: &[u8],
        signature: &[u8],
    ) -> Result<bool, String>;
}
