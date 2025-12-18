//! Tool execution result with cryptographic verification
//!
//! This module provides the `ToolResult` type which wraps tool output
//! with a SHA-256 hash for Merkle tree integration.
//!
//! # VEX Innovation
//!
//! Every tool execution result is automatically hashed, enabling:
//! - Cryptographic proof of tool execution
//! - Merkle tree integration for audit trails
//! - Tamper-evident logging
//!
//! # Security Considerations
//!
//! - Hash includes timestamp to prevent replay attacks
//! - Deterministic serialization ensures consistent hashing
//! - Output is NOT sanitized here (responsibility of the tool)

use serde::{Deserialize, Serialize};
use std::time::Duration;
use vex_core::Hash;

/// Result of a tool execution with cryptographic verification data.
///
/// # Example
///
/// ```
/// use vex_llm::ToolResult;
/// use serde_json::json;
/// use std::time::Duration;
///
/// let result = ToolResult::new(
///     "calculator",
///     &json!({"expression": "2+2"}),
///     json!({"result": 4}),
///     Duration::from_millis(5),
/// );
///
/// // Hash is automatically computed
/// assert!(!result.hash.to_string().is_empty());
/// assert_eq!(result.output["result"], 4);
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolResult {
    /// The output value from the tool
    pub output: serde_json::Value,

    /// SHA-256 hash of (tool_name + args + output + timestamp)
    /// Used for Merkle tree integration and verification
    pub hash: Hash,

    /// How long the tool took to execute
    #[serde(with = "duration_serde")]
    pub execution_time: Duration,

    /// Optional token count (for LLM-based tools)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tokens_used: Option<u32>,

    /// ISO 8601 timestamp of execution
    pub timestamp: String,

    /// Name of the tool that produced this result
    pub tool_name: String,
}

impl ToolResult {
    /// Create a new tool result with automatic hash computation.
    ///
    /// The hash is computed from a deterministic JSON representation of:
    /// - Tool name
    /// - Input arguments
    /// - Output value
    /// - Timestamp
    ///
    /// # Arguments
    ///
    /// * `tool_name` - Name of the tool
    /// * `args` - Input arguments that were passed to the tool
    /// * `output` - Result value from the tool
    /// * `execution_time` - How long execution took
    ///
    /// # Security
    ///
    /// The timestamp is captured at creation time and included in the hash
    /// to prevent replay attacks where an old result could be substituted.
    pub fn new(
        tool_name: &str,
        args: &serde_json::Value,
        output: serde_json::Value,
        execution_time: Duration,
    ) -> Self {
        let timestamp = chrono::Utc::now().to_rfc3339();

        // Create deterministic hash input
        // Using sorted object keys for consistency
        let hash_input = serde_json::json!({
            "args": args,
            "output": &output,
            "timestamp": &timestamp,
            "tool": tool_name,
        });

        // Compute SHA-256 hash
        let hash = Hash::digest(
            &serde_json::to_vec(&hash_input).unwrap_or_default()
        );

        Self {
            output,
            hash,
            execution_time,
            tokens_used: None,
            timestamp,
            tool_name: tool_name.to_string(),
        }
    }

    /// Add token usage information
    pub fn with_tokens(mut self, tokens: u32) -> Self {
        self.tokens_used = Some(tokens);
        self
    }

    /// Verify that the hash matches the content
    ///
    /// # Returns
    ///
    /// `true` if the hash is valid for the current content, `false` otherwise.
    /// A `false` result indicates potential tampering.
    pub fn verify(&self, args: &serde_json::Value) -> bool {
        let hash_input = serde_json::json!({
            "args": args,
            "output": &self.output,
            "timestamp": &self.timestamp,
            "tool": &self.tool_name,
        });

        let expected = Hash::digest(
            &serde_json::to_vec(&hash_input).unwrap_or_default()
        );

        self.hash == expected
    }

    /// Get execution time in milliseconds
    pub fn execution_ms(&self) -> u128 {
        self.execution_time.as_millis()
    }
}

/// Custom serialization for Duration
mod duration_serde {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        // Serialize as milliseconds for portability
        duration.as_millis().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let millis = u64::deserialize(deserializer)?;
        Ok(Duration::from_millis(millis))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_result_creation() {
        let args = json!({"expression": "1+1"});
        let result = ToolResult::new("calculator", &args, json!({"result": 2}), Duration::from_millis(10));

        assert_eq!(result.tool_name, "calculator");
        assert_eq!(result.output["result"], 2);
        assert!(!result.hash.to_string().is_empty());
        assert!(result.tokens_used.is_none());
    }

    #[test]
    fn test_hash_verification() {
        let args = json!({"expression": "2+2"});
        let result = ToolResult::new("calculator", &args, json!({"result": 4}), Duration::from_millis(5));

        // Valid verification
        assert!(result.verify(&args));

        // Invalid verification (different args)
        let different_args = json!({"expression": "3+3"});
        assert!(!result.verify(&different_args));
    }

    #[test]
    fn test_with_tokens() {
        let args = json!({});
        let result = ToolResult::new("llm_tool", &args, json!({"text": "hello"}), Duration::from_millis(100))
            .with_tokens(150);

        assert_eq!(result.tokens_used, Some(150));
    }

    #[test]
    fn test_serialization() {
        let args = json!({"x": 1});
        let result = ToolResult::new("test", &args, json!({"y": 2}), Duration::from_millis(50));

        let json = serde_json::to_string(&result).unwrap();
        let deserialized: ToolResult = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.tool_name, result.tool_name);
        assert_eq!(deserialized.output, result.output);
        assert_eq!(deserialized.hash, result.hash);
    }

    #[test]
    fn test_execution_ms() {
        let args = json!({});
        let result = ToolResult::new("test", &args, json!({}), Duration::from_millis(123));
        assert_eq!(result.execution_ms(), 123);
    }
}
