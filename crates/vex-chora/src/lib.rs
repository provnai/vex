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
    #[test]
    fn test_bridge_initialization() {
        // Placeholder for future bridge logic.
        // We verify that the test can run without panicking.
    }
}
