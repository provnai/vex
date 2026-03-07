use crate::client::AuthorityClient;
use serde::Serialize;

/// The Bridge manages the relationship between local agent intent
/// and external authority validation.
pub struct AuthorityBridge {
    pub client: Box<dyn AuthorityClient + Send + Sync>,
}

impl AuthorityBridge {
    pub fn new(client: Box<dyn AuthorityClient + Send + Sync>) -> Self {
        Self { client }
    }

    /// Canonicalize a payload for CHORA compliance (RFC 8785)
    pub fn canonicalize<T: Serialize>(payload: &T) -> Result<Vec<u8>, String> {
        serde_jcs::to_vec(payload).map_err(|e| format!("JCS Canonicalization failed: {}", e))
    }
}
