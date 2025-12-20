//! Tool definitions and execution framework for LLM function calling
//!
//! This module provides:
//! - [`ToolDefinition`] - Metadata describing a tool's interface
//! - [`Tool`] trait - The core interface all tools must implement
//! - [`ToolRegistry`] - Dynamic registration and lookup of tools
//! - [`Capability`] - Sandboxing hints for security isolation
//!
//! # VEX Innovation
//!
//! VEX tools are unique in that every execution is:
//! 1. Validated against JSON schema
//! 2. Executed with timeout protection
//! 3. Hashed into the Merkle audit chain
//!
//! This provides cryptographic proof of what tools were used.
//!
//! # Security Considerations
//!
//! - Tools declare required capabilities for sandboxing
//! - All tool execution has configurable timeouts (DoS protection)
//! - Input validation is mandatory before execution
//! - Registry prevents name collisions

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use crate::tool_error::ToolError;

/// Definition of a tool that can be called by an LLM.
///
/// This struct holds the metadata about a tool: its name, description,
/// and JSON Schema for parameters. It's used for:
/// - Generating OpenAI/Anthropic-compatible tool specifications
/// - Validating input arguments
/// - Documentation
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
    /// Must be unique within a registry
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

    /// Convert to Anthropic Claude-compatible tool format
    pub fn to_anthropic_format(&self) -> serde_json::Value {
        serde_json::json!({
            "name": self.name,
            "description": self.description,
            "input_schema": serde_json::from_str::<serde_json::Value>(self.parameters)
                .unwrap_or(serde_json::json!({}))
        })
    }
}

/// Capability requirements for sandboxing (future WASM isolation)
///
/// Tools declare what capabilities they need, enabling:
/// - Security auditing (what can this tool access?)
/// - Sandboxing decisions (can run in WASM if PureComputation only)
/// - Permission management
///
/// # Security Model
///
/// Capabilities follow the principle of least privilege.
/// Tools should request only what they need.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Pure computation, no I/O whatsoever
    /// Safe to run in any sandbox
    PureComputation,
    /// Requires network access (HTTP/TCP/UDP)
    Network,
    /// Requires filesystem access (read and/or write)
    FileSystem,
    /// Requires subprocess spawning
    Subprocess,
    /// Requires access to environment variables
    Environment,
    /// Requires cryptographic operations (signing, etc.)
    Cryptography,
}

/// The core Tool trait — interface all tools must implement.
///
/// Tools are the bridge between LLM decision-making and real-world actions.
/// Every tool must:
/// 1. Provide its definition (name, description, schema)
/// 2. Implement async execution
///
/// # Security
///
/// - `validate()` is called before `execute()` — reject bad input early
/// - `capabilities()` declares what the tool needs for sandboxing
/// - `timeout()` prevents DoS from hanging operations
///
/// # Example
///
/// ```ignore
/// use vex_llm::{Tool, ToolDefinition, ToolError, Capability};
/// use async_trait::async_trait;
///
/// pub struct MyTool {
///     definition: ToolDefinition,
/// }
///
/// #[async_trait]
/// impl Tool for MyTool {
///     fn definition(&self) -> &ToolDefinition {
///         &self.definition
///     }
///
///     async fn execute(&self, args: serde_json::Value) -> Result<serde_json::Value, ToolError> {
///         // Your implementation here
///         Ok(serde_json::json!({"status": "done"}))
///     }
/// }
/// ```
#[async_trait]
pub trait Tool: Send + Sync {
    /// Returns the tool's metadata (name, description, schema)
    fn definition(&self) -> &ToolDefinition;

    /// Execute the tool with given arguments.
    ///
    /// # Arguments
    ///
    /// * `args` - JSON value matching the tool's parameter schema
    ///
    /// # Returns
    ///
    /// * `Ok(Value)` - The tool's output as JSON
    /// * `Err(ToolError)` - If execution failed
    ///
    /// # Security
    ///
    /// Implementations should:
    /// - Validate all inputs (even if validate() passed)
    /// - Sanitize outputs (no raw filesystem paths, etc.)
    /// - Respect timeouts (use tokio::select! if needed)
    async fn execute(&self, args: Value) -> Result<Value, ToolError>;

    /// Validate arguments before execution.
    ///
    /// Called by ToolExecutor before execute(). Override to add
    /// custom validation beyond JSON schema checking.
    ///
    /// # Default
    ///
    /// Returns `Ok(())` — no additional validation.
    fn validate(&self, _args: &Value) -> Result<(), ToolError> {
        Ok(())
    }

    /// Required capabilities for sandboxing.
    ///
    /// # Default
    ///
    /// Returns `[PureComputation]` — safe for any sandbox.
    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation]
    }

    /// Execution timeout.
    ///
    /// # Default
    ///
    /// 30 seconds — adjust for long-running tools.
    fn timeout(&self) -> Duration {
        Duration::from_secs(30)
    }

    /// Whether the tool is currently available.
    ///
    /// # Default
    ///
    /// Always returns `true`. Override to implement availability checks
    /// (e.g., API health, rate limiting, maintenance windows).
    fn is_available(&self) -> bool {
        true
    }
}

/// Registry for dynamically registered tools.
///
/// The registry provides:
/// - O(1) tool lookup by name
/// - Collision detection (no duplicate names)
/// - Bulk operations (list, export)
/// - Format conversion for OpenAI/Anthropic
///
/// # Thread Safety
///
/// The registry itself is not thread-safe. Wrap in `Arc<RwLock<ToolRegistry>>`
/// if you need concurrent access. Tools within the registry are `Arc<dyn Tool>`.
///
/// # Example
///
/// ```ignore
/// let mut registry = ToolRegistry::new();
/// registry.register(Arc::new(MyCalculatorTool::new()));
///
/// if let Some(tool) = registry.get("calculator") {
///     let result = tool.execute(json!({"expr": "2+2"})).await?;
/// }
/// ```
#[derive(Default)]
pub struct ToolRegistry {
    tools: HashMap<String, Arc<dyn Tool>>,
}

impl ToolRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self::default()
    }

    /// Register a tool.
    ///
    /// # Returns
    ///
    /// `true` if the tool was added, `false` if a tool with that name already exists.
    ///
    /// # Security
    ///
    /// Name collisions are rejected to prevent tool impersonation attacks.
    pub fn register(&mut self, tool: Arc<dyn Tool>) -> bool {
        let name = tool.definition().name.to_string();
        if self.tools.contains_key(&name) {
            tracing::warn!("Tool '{}' already registered, skipping duplicate", name);
            return false;
        }
        self.tools.insert(name, tool);
        true
    }

    /// Register a tool, replacing any existing tool with the same name.
    ///
    /// # Security Warning
    ///
    /// Use with caution — this can replace trusted tools with untrusted ones.
    pub fn register_replace(&mut self, tool: Arc<dyn Tool>) {
        let name = tool.definition().name.to_string();
        if self.tools.contains_key(&name) {
            tracing::warn!("Replacing existing tool '{}'", name);
        }
        self.tools.insert(name, tool);
    }

    /// Get a tool by name
    pub fn get(&self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.get(name).cloned()
    }

    /// Check if a tool exists
    pub fn contains(&self, name: &str) -> bool {
        self.tools.contains_key(name)
    }

    /// Remove a tool by name
    pub fn remove(&mut self, name: &str) -> Option<Arc<dyn Tool>> {
        self.tools.remove(name)
    }

    /// List all tool names
    pub fn names(&self) -> Vec<&str> {
        self.tools.keys().map(|s| s.as_str()).collect()
    }

    /// List all tool definitions
    pub fn definitions(&self) -> Vec<&ToolDefinition> {
        self.tools.values().map(|t| t.definition()).collect()
    }

    /// Generate OpenAI-compatible tool list
    pub fn to_openai_format(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| t.definition().to_openai_format())
            .collect()
    }

    /// Generate Anthropic Claude-compatible tool list
    pub fn to_anthropic_format(&self) -> Vec<serde_json::Value> {
        self.tools
            .values()
            .map(|t| t.definition().to_anthropic_format())
            .collect()
    }

    /// Number of registered tools
    pub fn len(&self) -> usize {
        self.tools.len()
    }

    /// Check if registry is empty
    pub fn is_empty(&self) -> bool {
        self.tools.is_empty()
    }

    /// Get available tools only (where is_available() returns true)
    pub fn available(&self) -> Vec<Arc<dyn Tool>> {
        self.tools
            .values()
            .filter(|t| t.is_available())
            .cloned()
            .collect()
    }
}

impl std::fmt::Debug for ToolRegistry {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ToolRegistry")
            .field("tools", &self.names())
            .finish()
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

    #[test]
    fn test_anthropic_format() {
        let tool = ToolDefinition::new(
            "search",
            "Search the web",
            r#"{"type": "object", "properties": {"query": {"type": "string"}}}"#,
        );

        let json = tool.to_anthropic_format();
        assert_eq!(json["name"], "search");
        assert!(json.get("input_schema").is_some());
    }

    // Mock tool for testing
    struct MockTool {
        definition: ToolDefinition,
    }

    impl MockTool {
        fn new(name: &'static str) -> Self {
            Self {
                definition: ToolDefinition::new(name, "A mock tool", r#"{"type": "object"}"#),
            }
        }
    }

    #[async_trait]
    impl Tool for MockTool {
        fn definition(&self) -> &ToolDefinition {
            &self.definition
        }

        async fn execute(&self, _args: Value) -> Result<Value, ToolError> {
            Ok(serde_json::json!({"mock": true}))
        }
    }

    #[test]
    fn test_registry_basic() {
        let mut registry = ToolRegistry::new();
        assert!(registry.is_empty());

        let tool = Arc::new(MockTool::new("mock"));
        assert!(registry.register(tool));
        assert_eq!(registry.len(), 1);
        assert!(registry.contains("mock"));
    }

    #[test]
    fn test_registry_duplicate_rejection() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool::new("dup")));

        // Second registration should fail
        let duplicate = Arc::new(MockTool::new("dup"));
        assert!(!registry.register(duplicate));

        // Still only one tool
        assert_eq!(registry.len(), 1);
    }

    #[test]
    fn test_registry_lookup() {
        let mut registry = ToolRegistry::new();
        registry.register(Arc::new(MockTool::new("finder")));

        assert!(registry.get("finder").is_some());
        assert!(registry.get("nonexistent").is_none());
    }

    #[test]
    fn test_capability_enum() {
        let caps = [Capability::PureComputation, Capability::Network];
        assert!(caps.contains(&Capability::PureComputation));
        assert!(!caps.contains(&Capability::FileSystem));
    }
}
