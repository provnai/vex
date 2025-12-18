//! Hash tool for computing cryptographic hashes
//!
//! Computes SHA-256 and SHA-512 hashes of text input.
//!
//! # Security
//!
//! - Uses Rust's sha2 crate (no known vulnerabilities)
//! - Pure computation, no I/O
//! - Input length limited to prevent DoS

use async_trait::async_trait;
use serde_json::Value;
use sha2::{Sha256, Sha512, Digest};

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// Hash tool for computing SHA-256 and SHA-512 hashes.
///
/// # Example
///
/// ```ignore
/// use vex_llm::HashTool;
/// use vex_llm::Tool;
///
/// let hash = HashTool::new();
/// let result = hash.execute(json!({"text": "hello", "algorithm": "sha256"})).await?;
/// println!("{}", result["hash"]);
/// ```
pub struct HashTool {
    definition: ToolDefinition,
}

impl HashTool {
    /// Create a new hash tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "hash",
                "Compute cryptographic hash (SHA-256 or SHA-512) of text input.",
                r#"{
                    "type": "object",
                    "properties": {
                        "text": {
                            "type": "string",
                            "description": "Text to hash"
                        },
                        "algorithm": {
                            "type": "string",
                            "enum": ["sha256", "sha512"],
                            "default": "sha256",
                            "description": "Hash algorithm to use"
                        }
                    },
                    "required": ["text"]
                }"#,
            ),
        }
    }
}

impl Default for HashTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for HashTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation, Capability::Cryptography]
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        let text = args
            .get("text")
            .and_then(|t| t.as_str())
            .ok_or_else(|| {
                ToolError::invalid_args("hash", "Missing required field 'text'")
            })?;

        // Limit input size to prevent DoS (1MB max)
        if text.len() > 1_000_000 {
            return Err(ToolError::invalid_args(
                "hash",
                "Input text too large (max 1MB)",
            ));
        }

        // Validate algorithm if provided
        if let Some(algo) = args.get("algorithm").and_then(|a| a.as_str()) {
            if algo != "sha256" && algo != "sha512" {
                return Err(ToolError::invalid_args(
                    "hash",
                    format!("Invalid algorithm '{}'. Must be 'sha256' or 'sha512'", algo),
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let text = args["text"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_args("hash", "Missing 'text' field"))?;

        let algorithm = args
            .get("algorithm")
            .and_then(|a| a.as_str())
            .unwrap_or("sha256");

        let hash_hex = match algorithm {
            "sha512" => {
                let mut hasher = Sha512::new();
                hasher.update(text.as_bytes());
                hex::encode(hasher.finalize())
            }
            _ => {
                let mut hasher = Sha256::new();
                hasher.update(text.as_bytes());
                hex::encode(hasher.finalize())
            }
        };

        Ok(serde_json::json!({
            "hash": hash_hex,
            "algorithm": algorithm,
            "input_length": text.len()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_sha256_hash() {
        let tool = HashTool::new();
        let result = tool
            .execute(serde_json::json!({"text": "hello", "algorithm": "sha256"}))
            .await
            .unwrap();

        // Known SHA-256 of "hello"
        assert_eq!(
            result["hash"],
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
        assert_eq!(result["algorithm"], "sha256");
    }

    #[tokio::test]
    async fn test_sha512_hash() {
        let tool = HashTool::new();
        let result = tool
            .execute(serde_json::json!({"text": "hello", "algorithm": "sha512"}))
            .await
            .unwrap();

        assert_eq!(result["algorithm"], "sha512");
        // SHA-512 produces 128 hex characters
        assert_eq!(result["hash"].as_str().unwrap().len(), 128);
    }

    #[tokio::test]
    async fn test_default_algorithm() {
        let tool = HashTool::new();
        let result = tool
            .execute(serde_json::json!({"text": "test"}))
            .await
            .unwrap();

        // Default is SHA-256
        assert_eq!(result["algorithm"], "sha256");
    }

    #[tokio::test]
    async fn test_invalid_algorithm() {
        let tool = HashTool::new();
        let result = tool.validate(&serde_json::json!({"text": "hello", "algorithm": "md5"}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_missing_text() {
        let tool = HashTool::new();
        let result = tool.validate(&serde_json::json!({}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_empty_string() {
        let tool = HashTool::new();
        let result = tool
            .execute(serde_json::json!({"text": ""}))
            .await
            .unwrap();

        // SHA-256 of empty string
        assert_eq!(
            result["hash"],
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
    }
}
