//! Built-in tools for VEX agents
//!
//! This module provides a set of ready-to-use tools:
//!
//! - [`CalculatorTool`] - Evaluate mathematical expressions
//! - [`DateTimeTool`] - Get current date and time
//! - [`UuidTool`] - Generate UUIDs
//! - [`HashTool`] - Compute SHA-256/SHA-512 hashes
//! - [`RegexTool`] - Pattern matching and extraction
//! - [`JsonPathTool`] - JSON value extraction
//!
//! All built-in tools are pure computation (no network I/O) and safe for sandboxing.

mod calculator;
mod datetime;
mod hash;
mod json_path;
mod regex;
mod uuid_tool;

pub use calculator::CalculatorTool;
pub use datetime::DateTimeTool;
pub use hash::HashTool;
pub use json_path::JsonPathTool;
pub use regex::RegexTool;
pub use uuid_tool::UuidTool;

use std::sync::Arc;
use crate::tool::ToolRegistry;

/// Create a registry with all built-in tools pre-registered
///
/// # Example
///
/// ```ignore
/// let registry = vex_llm::tools::builtin_registry();
/// assert!(registry.contains("calculator"));
/// assert!(registry.contains("hash"));
/// ```
pub fn builtin_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Arc::new(CalculatorTool::new()));
    registry.register(Arc::new(DateTimeTool::new()));
    registry.register(Arc::new(UuidTool::new()));
    registry.register(Arc::new(HashTool::new()));
    registry.register(Arc::new(RegexTool::new()));
    registry.register(Arc::new(JsonPathTool::new()));
    registry
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_builtin_registry() {
        let registry = builtin_registry();
        
        assert!(registry.contains("calculator"));
        assert!(registry.contains("datetime"));
        assert!(registry.contains("uuid"));
        assert!(registry.contains("hash"));
        assert!(registry.contains("regex"));
        assert!(registry.contains("json_path"));
        assert_eq!(registry.len(), 6);
    }
}
