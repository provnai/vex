//! vex-chora: The Native Bridge to Neutral Authority.
//!
//! This crate provides the adapter logic required for AI agents to communicate
//! with the external CHORA witness network. It handles JCS serialization,
//! signature verification, and authority handshakes.

pub mod bridge;
pub mod client;

pub use bridge::AuthorityBridge;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::client::MockChoraClient;
    #[tokio::test]
    async fn test_bridge_handshake() {
        let bridge = AuthorityBridge::new(Box::new(MockChoraClient));
        let intent = vex_core::segment::IntentData {
            request_sha256: "8ee6010d905547c377c67e63559e989b8073b168f11a1ffefd092c7ca962076e"
                .to_string(),
            confidence: 0.95,
            capabilities: vec![],
        };

        let capsule = bridge.perform_handshake(intent).await.unwrap();

        // Verify segments are present
        assert_eq!(capsule.intent.confidence, 0.95);
        assert_eq!(capsule.authority.nonce, 42);
        assert_eq!(capsule.identity.aid, "mock-aid-01");

        // Verify signature is present (base64)
        assert!(!capsule.crypto.signature_b64.is_empty());

        // Verify composite hash generation
        let root = capsule.to_composite_hash().unwrap();
        assert!(!root.to_hex().is_empty());
    }
}
