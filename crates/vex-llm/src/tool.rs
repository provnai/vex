//! Tool definitions for LLM function calling

use serde::{Deserialize, Serialize};

/// Definition of a tool that can be called by an LLM.
///
/// Used with the `#[vex_tool]` macro to generate tool schemas.
///
/// # Example
/// ```
/// use vex_llm::ToolDefinition;
///
/// const SEARCH_TOOL: ToolDefinition = ToolDefinition {
///     name: "web_search",
///     description: "Search the web for information",
///     parameters: r#"{"type": "object", "properties": {"query": {"type": "string"}}}"#,
/// };
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    /// Name of the tool (used in function calling)
    pub name: &'static str,
    /// Human-readable description of what the tool does
    pub description: &'static str,
    /// JSON Schema for the tool's parameters
    pub parameters: &'static str,
}

impl ToolDefinition {
    /// Create a new tool definition
    pub const fn new(
        name: &'static str,
        description: &'static str,
        parameters: &'static str,
    ) -> Self {
        Self {
            name,
            description,
            parameters,
        }
    }

    /// Convert to OpenAI-compatible tool format
    pub fn to_openai_format(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "function",
            "function": {
                "name": self.name,
                "description": self.description,
                "parameters": serde_json::from_str::<serde_json::Value>(self.parameters)
                    .unwrap_or(serde_json::json!({}))
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tool_definition() {
        const TEST_TOOL: ToolDefinition =
            ToolDefinition::new("test_tool", "A test tool", r#"{"type": "object"}"#);

        assert_eq!(TEST_TOOL.name, "test_tool");
        assert_eq!(TEST_TOOL.description, "A test tool");
    }

    #[test]
    fn test_openai_format() {
        let tool = ToolDefinition::new(
            "search",
            "Search the web",
            r#"{"type": "object", "properties": {"query": {"type": "string"}}}"#,
        );

        let json = tool.to_openai_format();
        assert_eq!(json["type"], "function");
        assert_eq!(json["function"]["name"], "search");
    }
}
