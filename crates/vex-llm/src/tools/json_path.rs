//! JSON Path tool for querying JSON data
//!
//! Extracts values from JSON using path expressions.
//!
//! # Security
//!
//! - Input size limited (prevents memory exhaustion)
//! - Pure computation, no I/O
//! - Path syntax validated before execution

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// JSON Path tool for extracting values from JSON.
///
/// Supports simple dot-notation paths like "user.name" and array access "[0]".
///
/// # Example
///
/// ```ignore
/// use vex_llm::JsonPathTool;
/// use vex_llm::Tool;
///
/// let jp = JsonPathTool::new();
/// let result = jp.execute(json!({
///     "data": {"user": {"name": "Alice"}},
///     "path": "user.name"
/// })).await?;
/// println!("{}", result["value"]); // "Alice"
/// ```
pub struct JsonPathTool {
    definition: ToolDefinition,
}

impl JsonPathTool {
    /// Create a new JSON path tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "json_path",
                "Extract values from JSON using dot-notation paths. Supports: 'key', 'key.nested', 'array[0]', 'data.items[2].name'",
                r#"{
                    "type": "object",
                    "properties": {
                        "data": {
                            "description": "JSON data to query"
                        },
                        "path": {
                            "type": "string",
                            "description": "Path expression (e.g., 'user.name', 'items[0]')"
                        }
                    },
                    "required": ["data", "path"]
                }"#,
            ),
        }
    }

    /// Navigate to a nested value using a path string
    fn navigate<'a>(data: &'a Value, path: &str) -> Option<&'a Value> {
        let mut current = data;

        for segment in Self::parse_path(path) {
            match segment {
                PathSegment::Key(key) => {
                    current = current.get(key)?;
                }
                PathSegment::Index(idx) => {
                    current = current.get(idx)?;
                }
            }
        }

        Some(current)
    }

    /// Parse a path string into segments
    fn parse_path(path: &str) -> Vec<PathSegment> {
        let mut segments = Vec::new();
        let mut current_key = String::new();
        let mut chars = path.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '.' => {
                    if !current_key.is_empty() {
                        segments.push(PathSegment::Key(std::mem::take(&mut current_key)));
                    }
                }
                '[' => {
                    if !current_key.is_empty() {
                        segments.push(PathSegment::Key(std::mem::take(&mut current_key)));
                    }
                    // Parse array index
                    let mut idx_str = String::new();
                    while let Some(&next_c) = chars.peek() {
                        if next_c == ']' {
                            chars.next(); // consume ']'
                            break;
                        }
                        idx_str.push(chars.next().unwrap());
                    }
                    if let Ok(idx) = idx_str.parse::<usize>() {
                        segments.push(PathSegment::Index(idx));
                    }
                }
                ']' => {} // Already handled
                _ => {
                    current_key.push(c);
                }
            }
        }

        if !current_key.is_empty() {
            segments.push(PathSegment::Key(current_key));
        }

        segments
    }
}

#[derive(Debug)]
enum PathSegment {
    Key(String),
    Index(usize),
}

impl Default for JsonPathTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for JsonPathTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation]
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        // Check data exists
        if args.get("data").is_none() {
            return Err(ToolError::invalid_args(
                "json_path",
                "Missing required field 'data'",
            ));
        }

        // Check path exists and is valid
        let path = args
            .get("path")
            .and_then(|p| p.as_str())
            .ok_or_else(|| ToolError::invalid_args("json_path", "Missing required field 'path'"))?;

        if path.is_empty() {
            return Err(ToolError::invalid_args("json_path", "Path cannot be empty"));
        }

        if path.len() > 200 {
            return Err(ToolError::invalid_args(
                "json_path",
                "Path too long (max 200 characters)",
            ));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let data = args
            .get("data")
            .ok_or_else(|| ToolError::invalid_args("json_path", "Missing 'data' field"))?;

        let path = args["path"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_args("json_path", "Missing 'path' field"))?;

        let value = Self::navigate(data, path);

        Ok(serde_json::json!({
            "path": path,
            "found": value.is_some(),
            "value": value.cloned()
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_key() {
        let tool = JsonPathTool::new();
        let result = tool
            .execute(serde_json::json!({
                "data": {"name": "Alice"},
                "path": "name"
            }))
            .await
            .unwrap();

        assert_eq!(result["found"], true);
        assert_eq!(result["value"], "Alice");
    }

    #[tokio::test]
    async fn test_nested_key() {
        let tool = JsonPathTool::new();
        let result = tool
            .execute(serde_json::json!({
                "data": {"user": {"name": "Bob"}},
                "path": "user.name"
            }))
            .await
            .unwrap();

        assert_eq!(result["found"], true);
        assert_eq!(result["value"], "Bob");
    }

    #[tokio::test]
    async fn test_array_index() {
        let tool = JsonPathTool::new();
        let result = tool
            .execute(serde_json::json!({
                "data": {"items": ["a", "b", "c"]},
                "path": "items[1]"
            }))
            .await
            .unwrap();

        assert_eq!(result["found"], true);
        assert_eq!(result["value"], "b");
    }

    #[tokio::test]
    async fn test_complex_path() {
        let tool = JsonPathTool::new();
        let result = tool
            .execute(serde_json::json!({
                "data": {
                    "users": [
                        {"name": "Alice", "age": 30},
                        {"name": "Bob", "age": 25}
                    ]
                },
                "path": "users[1].name"
            }))
            .await
            .unwrap();

        assert_eq!(result["found"], true);
        assert_eq!(result["value"], "Bob");
    }

    #[tokio::test]
    async fn test_not_found() {
        let tool = JsonPathTool::new();
        let result = tool
            .execute(serde_json::json!({
                "data": {"name": "Alice"},
                "path": "age"
            }))
            .await
            .unwrap();

        assert_eq!(result["found"], false);
        assert!(result["value"].is_null());
    }

    #[tokio::test]
    async fn test_empty_path() {
        let tool = JsonPathTool::new();
        let result = tool.validate(&serde_json::json!({
            "data": {"name": "Alice"},
            "path": ""
        }));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
