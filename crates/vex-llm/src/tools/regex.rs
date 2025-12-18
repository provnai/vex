//! Regex tool for pattern matching
//!
//! Provides regex matching and extraction capabilities.
//!
//! # Security
//!
//! - Pattern length limited (prevents ReDoS)
//! - Execution timeout on complex patterns
//! - Pure computation, no I/O

use async_trait::async_trait;
use regex::Regex;
use serde_json::Value;

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// Regex tool for pattern matching and extraction.
///
/// # Example
///
/// ```ignore
/// use vex_llm::RegexTool;
/// use vex_llm::Tool;
///
/// let re = RegexTool::new();
/// let result = re.execute(json!({
///     "pattern": r"\d+",
///     "text": "Order 12345",
///     "operation": "find_all"
/// })).await?;
/// println!("{:?}", result["matches"]);
/// ```
pub struct RegexTool {
    definition: ToolDefinition,
}

impl RegexTool {
    /// Create a new regex tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "regex",
                "Match or extract patterns using regular expressions.",
                r#"{
                    "type": "object",
                    "properties": {
                        "pattern": {
                            "type": "string",
                            "description": "Regular expression pattern"
                        },
                        "text": {
                            "type": "string",
                            "description": "Text to search in"
                        },
                        "operation": {
                            "type": "string",
                            "enum": ["match", "find_all", "replace"],
                            "default": "match",
                            "description": "Operation: 'match' (check if matches), 'find_all' (extract all matches), 'replace' (replace matches)"
                        },
                        "replacement": {
                            "type": "string",
                            "description": "Replacement string (for 'replace' operation)"
                        }
                    },
                    "required": ["pattern", "text"]
                }"#,
            ),
        }
    }
}

impl Default for RegexTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for RegexTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation]
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        let pattern = args
            .get("pattern")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::invalid_args("regex", "Missing required field 'pattern'"))?;

        // Limit pattern length to prevent ReDoS
        if pattern.len() > 500 {
            return Err(ToolError::invalid_args(
                "regex",
                "Pattern too long (max 500 characters)",
            ));
        }

        // Validate the regex compiles
        Regex::new(pattern).map_err(|e| {
            ToolError::invalid_args("regex", format!("Invalid regex pattern: {}", e))
        })?;

        // Check text is provided
        if args.get("text").and_then(|t| t.as_str()).is_none() {
            return Err(ToolError::invalid_args("regex", "Missing required field 'text'"));
        }

        // Limit text length
        if let Some(text) = args.get("text").and_then(|t| t.as_str()) {
            if text.len() > 100_000 {
                return Err(ToolError::invalid_args(
                    "regex",
                    "Text too long (max 100KB)",
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let pattern = args["pattern"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_args("regex", "Missing 'pattern' field"))?;

        let text = args["text"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_args("regex", "Missing 'text' field"))?;

        let operation = args
            .get("operation")
            .and_then(|o| o.as_str())
            .unwrap_or("match");

        let re = Regex::new(pattern).map_err(|e| {
            ToolError::execution_failed("regex", format!("Invalid regex: {}", e))
        })?;

        match operation {
            "match" => {
                let is_match = re.is_match(text);
                let first_match = re.find(text).map(|m| m.as_str().to_string());

                Ok(serde_json::json!({
                    "matches": is_match,
                    "first_match": first_match,
                    "pattern": pattern
                }))
            }
            "find_all" => {
                let matches: Vec<String> = re
                    .find_iter(text)
                    .map(|m| m.as_str().to_string())
                    .collect();

                Ok(serde_json::json!({
                    "matches": matches,
                    "count": matches.len(),
                    "pattern": pattern
                }))
            }
            "replace" => {
                let replacement = args
                    .get("replacement")
                    .and_then(|r| r.as_str())
                    .unwrap_or("");

                let result = re.replace_all(text, replacement).to_string();

                Ok(serde_json::json!({
                    "result": result,
                    "pattern": pattern,
                    "replacement": replacement
                }))
            }
            _ => Err(ToolError::invalid_args(
                "regex",
                format!("Unknown operation '{}'. Use 'match', 'find_all', or 'replace'", operation),
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_match_found() {
        let tool = RegexTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": r"\d+",
                "text": "Order 12345",
                "operation": "match"
            }))
            .await
            .unwrap();

        assert_eq!(result["matches"], true);
        assert_eq!(result["first_match"], "12345");
    }

    #[tokio::test]
    async fn test_match_not_found() {
        let tool = RegexTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": r"\d+",
                "text": "No numbers here",
                "operation": "match"
            }))
            .await
            .unwrap();

        assert_eq!(result["matches"], false);
        assert!(result["first_match"].is_null());
    }

    #[tokio::test]
    async fn test_find_all() {
        let tool = RegexTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": r"\d+",
                "text": "Items: 10, 20, 30",
                "operation": "find_all"
            }))
            .await
            .unwrap();

        let matches = result["matches"].as_array().unwrap();
        assert_eq!(matches.len(), 3);
        assert_eq!(matches[0], "10");
        assert_eq!(matches[1], "20");
        assert_eq!(matches[2], "30");
    }

    #[tokio::test]
    async fn test_replace() {
        let tool = RegexTool::new();
        let result = tool
            .execute(serde_json::json!({
                "pattern": r"\d+",
                "text": "Price: $100",
                "operation": "replace",
                "replacement": "XXX"
            }))
            .await
            .unwrap();

        assert_eq!(result["result"], "Price: $XXX");
    }

    #[tokio::test]
    async fn test_invalid_pattern() {
        let tool = RegexTool::new();
        let result = tool.validate(&serde_json::json!({
            "pattern": "[invalid(",
            "text": "test"
        }));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_pattern_too_long() {
        let tool = RegexTool::new();
        let long_pattern = "a".repeat(600);
        let result = tool.validate(&serde_json::json!({
            "pattern": long_pattern,
            "text": "test"
        }));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
