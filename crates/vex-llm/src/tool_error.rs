//! Structured error types for tool execution
//!
//! This module provides precise, actionable error types for the VEX Tool System.
//! Uses `thiserror` for automatic `std::error::Error` implementation.
//!
//! # Security Considerations
//!
//! - Error messages sanitize sensitive data (no raw args in messages)
//! - Timeout errors prevent DoS from hanging tools
//! - Validation errors provide safe feedback without exposing internals

use thiserror::Error;

/// Error types for tool execution with precise variants for each failure mode.
///
/// # Example
///
/// ```
/// use vex_llm::ToolError;
///
/// let err = ToolError::not_found("unknown_tool");
/// assert!(err.to_string().contains("unknown_tool"));
/// ```
#[derive(Debug, Error)]
pub enum ToolError {
    /// Tool not found in registry
    #[error("Tool '{name}' not found in registry")]
    NotFound {
        /// Name of the tool that was requested
        name: String,
    },

    /// Invalid arguments provided to tool
    #[error("Invalid arguments for '{tool}': {reason}")]
    InvalidArguments {
        /// Name of the tool
        tool: String,
        /// Human-readable reason for validation failure
        reason: String,
    },

    /// Tool execution failed
    #[error("Execution of '{tool}' failed: {message}")]
    ExecutionFailed {
        /// Name of the tool
        tool: String,
        /// Error message (sanitized)
        message: String,
    },

    /// Tool execution exceeded timeout
    #[error("Tool '{tool}' timed out after {timeout_ms}ms")]
    Timeout {
        /// Name of the tool
        tool: String,
        /// Timeout in milliseconds
        timeout_ms: u64,
    },

    /// JSON serialization/deserialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Audit logging failed (non-fatal, logged but doesn't stop execution)
    #[error("Audit logging failed: {0}")]
    AuditFailed(String),

    /// Tool is disabled or unavailable
    #[error("Tool '{name}' is currently unavailable: {reason}")]
    Unavailable {
        /// Name of the tool
        name: String,
        /// Reason for unavailability
        reason: String,
    },
}

impl ToolError {
    /// Create a NotFound error
    ///
    /// # Example
    /// ```
    /// use vex_llm::ToolError;
    /// let err = ToolError::not_found("my_tool");
    /// ```
    pub fn not_found(name: impl Into<String>) -> Self {
        Self::NotFound { name: name.into() }
    }

    /// Create an InvalidArguments error with context
    ///
    /// # Security Note
    /// The `reason` should not contain raw user input to prevent information leakage
    pub fn invalid_args(tool: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::InvalidArguments {
            tool: tool.into(),
            reason: reason.into(),
        }
    }

    /// Create an ExecutionFailed error
    ///
    /// # Security Note
    /// The `message` should be sanitized before passing to this constructor
    pub fn execution_failed(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::ExecutionFailed {
            tool: tool.into(),
            message: message.into(),
        }
    }

    /// Create a Timeout error
    pub fn timeout(tool: impl Into<String>, timeout_ms: u64) -> Self {
        Self::Timeout {
            tool: tool.into(),
            timeout_ms,
        }
    }

    /// Create an Unavailable error
    pub fn unavailable(name: impl Into<String>, reason: impl Into<String>) -> Self {
        Self::Unavailable {
            name: name.into(),
            reason: reason.into(),
        }
    }

    /// Check if this error is recoverable (can retry)
    pub fn is_retryable(&self) -> bool {
        matches!(self, Self::Timeout { .. } | Self::Unavailable { .. })
    }

    /// Check if this error should be logged to audit trail
    pub fn should_audit(&self) -> bool {
        // All errors except audit failures themselves should be logged
        !matches!(self, Self::AuditFailed(_))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = ToolError::not_found("calculator");
        assert!(err.to_string().contains("calculator"));
        assert!(err.to_string().contains("not found"));
    }

    #[test]
    fn test_invalid_args() {
        let err = ToolError::invalid_args("datetime", "Missing timezone field");
        assert!(err.to_string().contains("datetime"));
        assert!(err.to_string().contains("Missing timezone"));
    }

    #[test]
    fn test_retryable() {
        assert!(ToolError::timeout("test", 1000).is_retryable());
        assert!(ToolError::unavailable("test", "maintenance").is_retryable());
        assert!(!ToolError::not_found("test").is_retryable());
    }

    #[test]
    fn test_should_audit() {
        assert!(ToolError::not_found("test").should_audit());
        assert!(!ToolError::AuditFailed("db error".into()).should_audit());
    }
}
