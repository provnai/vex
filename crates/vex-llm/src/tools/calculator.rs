//! Calculator tool for evaluating mathematical expressions
//!
//! Uses the `meval` crate for safe expression evaluation.
//!
//! # Security
//!
//! - Only arithmetic operations are allowed (no arbitrary code execution)
//! - Does not access filesystem, network, or environment
//! - Pure computation: safe for any sandbox
//!
//! # Supported Operations
//!
//! - Basic arithmetic: `+`, `-`, `*`, `/`, `^` (power)
//! - Parentheses: `(2 + 3) * 4`
//! - Functions: `sqrt()`, `sin()`, `cos()`, `tan()`, `log()`, `exp()`, `abs()`
//! - Constants: `pi`, `e`

use async_trait::async_trait;
use serde_json::Value;

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// Calculator tool for evaluating mathematical expressions.
///
/// # Example
///
/// ```ignore
/// use vex_llm::CalculatorTool;
/// use vex_llm::Tool;
///
/// let calc = CalculatorTool::new();
/// let result = calc.execute(json!({"expression": "2 + 3 * 4"})).await?;
/// assert_eq!(result["result"], 14.0);
/// ```
pub struct CalculatorTool {
    definition: ToolDefinition,
}

impl CalculatorTool {
    /// Create a new calculator tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "calculator",
                "Evaluate mathematical expressions. Supports arithmetic operators (+, -, *, /, ^), \
                 functions (sqrt, sin, cos, tan, log, exp, abs), and constants (pi, e).",
                r#"{
                    "type": "object",
                    "properties": {
                        "expression": {
                            "type": "string",
                            "description": "Mathematical expression to evaluate, e.g. '2 + 3 * 4' or 'sqrt(16)'"
                        }
                    },
                    "required": ["expression"]
                }"#,
            ),
        }
    }
}

impl Default for CalculatorTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for CalculatorTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation] // Safe: no I/O
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        // Check required field exists
        let expr = args
            .get("expression")
            .and_then(|e| e.as_str())
            .ok_or_else(|| {
                ToolError::invalid_args("calculator", "Missing required field 'expression'")
            })?;

        // Basic length check to prevent DoS
        if expr.len() > 1000 {
            return Err(ToolError::invalid_args(
                "calculator",
                "Expression too long (max 1000 characters)",
            ));
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let expr = args["expression"]
            .as_str()
            .ok_or_else(|| ToolError::invalid_args("calculator", "Missing 'expression' field"))?;

        // Evaluate the expression using meval
        // meval is safe: only arithmetic, no arbitrary code execution
        let result = meval::eval_str(expr).map_err(|e| {
            ToolError::execution_failed(
                "calculator",
                format!("Failed to evaluate expression: {}", e),
            )
        })?;

        // Check for NaN or Infinity
        if result.is_nan() {
            return Err(ToolError::execution_failed(
                "calculator",
                "Result is not a number (NaN)",
            ));
        }
        if result.is_infinite() {
            return Err(ToolError::execution_failed(
                "calculator",
                "Result is infinite (division by zero or overflow)",
            ));
        }

        Ok(serde_json::json!({
            "expression": expr,
            "result": result
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_arithmetic() {
        let calc = CalculatorTool::new();
        let result = calc
            .execute(serde_json::json!({"expression": "2 + 3 * 4"}))
            .await
            .unwrap();

        assert_eq!(result["result"], 14.0);
    }

    #[tokio::test]
    async fn test_with_parentheses() {
        let calc = CalculatorTool::new();
        let result = calc
            .execute(serde_json::json!({"expression": "(2 + 3) * 4"}))
            .await
            .unwrap();

        assert_eq!(result["result"], 20.0);
    }

    #[tokio::test]
    async fn test_functions() {
        let calc = CalculatorTool::new();
        let result = calc
            .execute(serde_json::json!({"expression": "sqrt(16)"}))
            .await
            .unwrap();

        assert_eq!(result["result"], 4.0);
    }

    #[tokio::test]
    async fn test_constants() {
        let calc = CalculatorTool::new();
        let result = calc
            .execute(serde_json::json!({"expression": "pi"}))
            .await
            .unwrap();

        let pi = result["result"].as_f64().unwrap();
        assert!((pi - std::f64::consts::PI).abs() < 0.0001);
    }

    #[tokio::test]
    async fn test_invalid_expression() {
        let calc = CalculatorTool::new();
        let result = calc
            .execute(serde_json::json!({"expression": "invalid ++ syntax"}))
            .await;

        assert!(matches!(result, Err(ToolError::ExecutionFailed { .. })));
    }

    #[tokio::test]
    async fn test_missing_expression() {
        let calc = CalculatorTool::new();
        let result = calc.validate(&serde_json::json!({}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_division_by_zero() {
        let calc = CalculatorTool::new();
        let result = calc.execute(serde_json::json!({"expression": "1/0"})).await;

        assert!(matches!(result, Err(ToolError::ExecutionFailed { .. })));
    }

    #[tokio::test]
    async fn test_expression_too_long() {
        let calc = CalculatorTool::new();
        let long_expr = "1+".repeat(600);
        let result = calc.validate(&serde_json::json!({"expression": long_expr}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
