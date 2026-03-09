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
            id: "test-agent".to_string(),
            goal: "TASK_EXECUTION".to_string(),
            description: None,
            ticket_id: None,
            constraints: vec![],
            acceptance_criteria: vec![],
            status: "OPEN".to_string(),
            created_at: "2024-01-01T00:00:00Z".to_string(),
            closed_at: None,
        };

        let capsule = bridge.perform_handshake(intent).await.unwrap();

        // Verify segments are present
        assert_eq!(capsule.intent.id, "test-agent");
        assert_eq!(capsule.authority.nonce, 42);
        assert_eq!(capsule.identity.agent, "mock-hardware-id-01");

        // Verify signature is present (hex string)
        assert!(!capsule.chora_signature.is_empty());

        // Verify composite hash generation
        let root = capsule.to_composite_hash().unwrap();
        assert!(!root.to_hex().is_empty());
    }
}
