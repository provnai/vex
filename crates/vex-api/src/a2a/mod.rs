//! A2A (Agent-to-Agent) Protocol support for VEX
//!
//! This module provides types and handlers for the Google A2A protocol,
//! enabling VEX agents to communicate with other AI agents.
//!
//! # Security
//!
//! - OAuth 2.0 / JWT authentication
//! - mTLS for agent-to-agent connections
//! - All task responses are Merkle-verified
//! - Nonce + timestamp for replay protection
//!
//! # References
//!
//! - [A2A Protocol Spec](https://a2aprotocol.ai)
//! - [Google A2A Blog](https://developers.googleblog.com/en/a2a-agent-protocol/)

pub mod agent_card;
pub mod handler;
pub mod task;

pub use agent_card::{AgentCard, AuthConfig, Skill};
pub use task::{TaskRequest, TaskResponse, TaskStatus};
