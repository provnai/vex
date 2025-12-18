//! MCP (Model Context Protocol) client integration
//!
//! This module provides integration with MCP servers, allowing VEX agents
//! to use external tools exposed via the Model Context Protocol.
//!
//! # Security Considerations
//!
//! - **OAuth 2.1**: Authentication for remote MCP servers
//! - **TLS**: All remote connections use HTTPS/WSS
//! - **Merkle Hashing**: All MCP results are hashed for audit trail
//! - **Timeouts**: Connection and execution timeouts
//! - **Input Validation**: Arguments validated before sending
//!
//! # Example
//!
//! ```ignore
//! use vex_llm::mcp::McpClient;
//!
//! let client = McpClient::connect("ws://localhost:8080").await?;
//! let tools = client.list_tools().await?;
//! let result = client.call_tool("query", json!({"sql": "SELECT 1"})).await?;
//! ```

pub mod client;
pub mod types;

pub use client::McpClient;
pub use types::{McpToolInfo, McpError, McpConfig};
