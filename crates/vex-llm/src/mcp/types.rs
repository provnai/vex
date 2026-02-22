//! MCP types and configuration
//!
//! Core types for MCP protocol integration.

use serde::{Deserialize, Serialize};
use std::time::Duration;

/// MCP client configuration
///
/// # Security
///
/// - `connect_timeout`: Prevents hanging connections (DoS)
/// - `request_timeout`: Prevents slow loris attacks
/// - `max_response_size`: Limits memory usage
/// - `require_tls`: Enforces encrypted connections
#[derive(Debug, Clone)]
pub struct McpConfig {
    /// Connection timeout
    pub connect_timeout: Duration,
    /// Request timeout for tool calls
    pub request_timeout: Duration,
    /// Maximum response size in bytes
    pub max_response_size: usize,
    /// Require TLS for connections (enforced for non-localhost)
    pub require_tls: bool,
    /// OAuth 2.1 token (if authentication required)
    pub auth_token: Option<String>,
}

impl Default for McpConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(10),
            request_timeout: Duration::from_secs(30),
            max_response_size: 10 * 1024 * 1024, // 10MB
            require_tls: true,
            auth_token: None,
        }
    }
}

impl McpConfig {
    /// Create config with authentication
    pub fn with_auth(mut self, token: impl Into<String>) -> Self {
        self.auth_token = Some(token.into());
        self
    }

    /// Allow non-TLS connections (for local development only)
    ///
    /// # Security Warning
    /// Only use this for localhost connections during development
    pub fn allow_insecure(mut self) -> Self {
        self.require_tls = false;
        self
    }
}

/// Information about an MCP tool
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpToolInfo {
    /// Tool name
    pub name: String,
    /// Tool description
    pub description: String,
    /// JSON Schema for parameters
    pub input_schema: serde_json::Value,
}

/// MCP-specific errors
#[derive(Debug, thiserror::Error)]
pub enum McpError {
    /// Connection failed
    #[error("Failed to connect to MCP server: {0}")]
    ConnectionFailed(String),

    /// Authentication failed
    #[error("Authentication failed: {0}")]
    AuthenticationFailed(String),

    /// Tool not found on server
    #[error("Tool '{0}' not found on MCP server")]
    ToolNotFound(String),

    /// Tool execution failed
    #[error("MCP tool execution failed: {0}")]
    ExecutionFailed(String),

    /// Response too large
    #[error("Response exceeded maximum size ({0} bytes)")]
    ResponseTooLarge(usize),

    /// Timeout
    #[error("Operation timed out after {0:?}")]
    Timeout(Duration),

    /// TLS required but not available
    #[error("TLS required for remote connections")]
    TlsRequired,

    /// Protocol error
    #[error("MCP protocol error: {0}")]
    ProtocolError(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),
}

impl McpError {
    /// Check if error is recoverable
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::ConnectionFailed(_) | Self::Timeout(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_default() {
        let config = McpConfig::default();
        assert_eq!(config.connect_timeout, Duration::from_secs(10));
        assert!(config.require_tls);
        assert!(config.auth_token.is_none());
    }

    #[test]
    fn test_config_with_auth() {
        let config = McpConfig::default().with_auth("token123");
        assert_eq!(config.auth_token, Some("token123".to_string()));
    }

    #[test]
    fn test_error_retryable() {
        assert!(McpError::ConnectionFailed("test".into()).is_retryable());
        assert!(McpError::Timeout(Duration::from_secs(1)).is_retryable());
        assert!(!McpError::ToolNotFound("test".into()).is_retryable());
    }
}
