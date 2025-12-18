//! UUID generation tool
//!
//! Uses the `uuid` crate for generating UUIDs.
//!
//! # Security
//!
//! - Uses cryptographically secure random number generator
//! - Only generates UUIDs, no I/O operations
//! - Pure computation: safe for any sandbox

use async_trait::async_trait;
use serde_json::Value;
use uuid::Uuid;

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// UUID generation tool.
///
/// Generates version 4 (random) UUIDs using a cryptographically
/// secure random number generator.
///
/// # Example
///
/// ```ignore
/// use vex_llm::UuidTool;
/// use vex_llm::Tool;
///
/// let uuid_tool = UuidTool::new();
/// let result = uuid_tool.execute(json!({})).await?;
/// println!("{}", result["uuid"]);
/// ```
pub struct UuidTool {
    definition: ToolDefinition,
}

impl UuidTool {
    /// Create a new UUID tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "uuid",
                "Generate a universally unique identifier (UUID v4).",
                r#"{
                    "type": "object",
                    "properties": {
                        "count": {
                            "type": "integer",
                            "minimum": 1,
                            "maximum": 100,
                            "default": 1,
                            "description": "Number of UUIDs to generate (1-100)"
                        },
                        "format": {
                            "type": "string",
                            "enum": ["hyphenated", "simple", "urn"],
                            "default": "hyphenated",
                            "description": "Output format: 'hyphenated' (550e8400-e29b-41d4-a716-446655440000), 'simple' (550e8400e29b41d4a716446655440000), 'urn' (urn:uuid:550e8400-e29b-41d4-a716-446655440000)"
                        }
                    }
                }"#,
            ),
        }
    }
}

impl Default for UuidTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for UuidTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        // Uses cryptographic RNG, tag it accordingly
        vec![Capability::PureComputation, Capability::Cryptography]
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        // Validate count if provided
        if let Some(count) = args.get("count") {
            if let Some(n) = count.as_i64() {
                if n < 1 || n > 100 {
                    return Err(ToolError::invalid_args(
                        "uuid",
                        format!("Count must be between 1 and 100, got {}", n),
                    ));
                }
            } else if !count.is_null() {
                return Err(ToolError::invalid_args("uuid", "Count must be an integer"));
            }
        }

        // Validate format if provided
        if let Some(fmt) = args.get("format").and_then(|v| v.as_str()) {
            if fmt != "hyphenated" && fmt != "simple" && fmt != "urn" {
                return Err(ToolError::invalid_args(
                    "uuid",
                    format!(
                        "Invalid format '{}'. Must be 'hyphenated', 'simple', or 'urn'",
                        fmt
                    ),
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let count = args.get("count").and_then(|v| v.as_i64()).unwrap_or(1) as usize;

        let format = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("hyphenated");

        let uuids: Vec<String> = (0..count)
            .map(|_| {
                let uuid = Uuid::new_v4();
                match format {
                    "simple" => uuid.simple().to_string(),
                    "urn" => uuid.urn().to_string(),
                    _ => uuid.hyphenated().to_string(),
                }
            })
            .collect();

        if count == 1 {
            Ok(serde_json::json!({
                "uuid": uuids[0],
                "format": format,
                "version": 4
            }))
        } else {
            Ok(serde_json::json!({
                "uuids": uuids,
                "count": count,
                "format": format,
                "version": 4
            }))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_generate_single_uuid() {
        let tool = UuidTool::new();
        let result = tool.execute(serde_json::json!({})).await.unwrap();

        assert!(result["uuid"].is_string());
        let uuid = result["uuid"].as_str().unwrap();

        // Validate UUID format (hyphenated has 36 chars)
        assert_eq!(uuid.len(), 36);
        assert!(uuid.contains('-'));
    }

    #[tokio::test]
    async fn test_generate_multiple_uuids() {
        let tool = UuidTool::new();
        let result = tool.execute(serde_json::json!({"count": 5})).await.unwrap();

        let uuids = result["uuids"].as_array().unwrap();
        assert_eq!(uuids.len(), 5);

        // All UUIDs should be unique
        let mut unique: std::collections::HashSet<&str> = std::collections::HashSet::new();
        for uuid in uuids {
            unique.insert(uuid.as_str().unwrap());
        }
        assert_eq!(unique.len(), 5);
    }

    #[tokio::test]
    async fn test_simple_format() {
        let tool = UuidTool::new();
        let result = tool
            .execute(serde_json::json!({"format": "simple"}))
            .await
            .unwrap();

        let uuid = result["uuid"].as_str().unwrap();
        assert_eq!(uuid.len(), 32); // No hyphens
        assert!(!uuid.contains('-'));
    }

    #[tokio::test]
    async fn test_urn_format() {
        let tool = UuidTool::new();
        let result = tool
            .execute(serde_json::json!({"format": "urn"}))
            .await
            .unwrap();

        let uuid = result["uuid"].as_str().unwrap();
        assert!(uuid.starts_with("urn:uuid:"));
    }

    #[tokio::test]
    async fn test_invalid_count() {
        let tool = UuidTool::new();
        let result = tool.validate(&serde_json::json!({"count": 500}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_invalid_format() {
        let tool = UuidTool::new();
        let result = tool.validate(&serde_json::json!({"format": "invalid"}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_uuid_version() {
        let tool = UuidTool::new();
        let result = tool.execute(serde_json::json!({})).await.unwrap();

        assert_eq!(result["version"], 4);
    }
}
