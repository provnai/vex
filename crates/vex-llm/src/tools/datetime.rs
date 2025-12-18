//! DateTime tool for getting current date and time
//!
//! Uses the `chrono` crate for time operations.
//!
//! # Security
//!
//! - Only reads the system clock (no I/O)
//! - Does not modify any state
//! - Pure computation: safe for any sandbox

use async_trait::async_trait;
use chrono::{Local, Utc};
use serde_json::Value;

use crate::tool::{Capability, Tool, ToolDefinition};
use crate::tool_error::ToolError;

/// DateTime tool for retrieving current date and time.
///
/// # Example
///
/// ```ignore
/// use vex_llm::DateTimeTool;
/// use vex_llm::Tool;
///
/// let dt = DateTimeTool::new();
/// let result = dt.execute(json!({"timezone": "utc"})).await?;
/// println!("{}", result["datetime"]);
/// ```
pub struct DateTimeTool {
    definition: ToolDefinition,
}

impl DateTimeTool {
    /// Create a new datetime tool
    pub fn new() -> Self {
        Self {
            definition: ToolDefinition::new(
                "datetime",
                "Get the current date and time in various formats and timezones.",
                r#"{
                    "type": "object",
                    "properties": {
                        "timezone": {
                            "type": "string",
                            "enum": ["utc", "local"],
                            "default": "utc",
                            "description": "Timezone: 'utc' for UTC or 'local' for system local time"
                        },
                        "format": {
                            "type": "string",
                            "description": "strftime format string. Default: '%Y-%m-%d %H:%M:%S'. Common formats: '%Y-%m-%d' (date only), '%H:%M:%S' (time only), '%Y-%m-%dT%H:%M:%SZ' (ISO 8601)"
                        }
                    }
                }"#,
            ),
        }
    }
}

impl Default for DateTimeTool {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl Tool for DateTimeTool {
    fn definition(&self) -> &ToolDefinition {
        &self.definition
    }

    fn capabilities(&self) -> Vec<Capability> {
        vec![Capability::PureComputation] // Just reads system clock
    }

    fn validate(&self, args: &Value) -> Result<(), ToolError> {
        // Validate timezone if provided
        if let Some(tz) = args.get("timezone").and_then(|v| v.as_str()) {
            if tz != "utc" && tz != "local" {
                return Err(ToolError::invalid_args(
                    "datetime",
                    format!("Invalid timezone '{}'. Must be 'utc' or 'local'", tz),
                ));
            }
        }

        // Validate format string length
        if let Some(fmt) = args.get("format").and_then(|v| v.as_str()) {
            if fmt.len() > 100 {
                return Err(ToolError::invalid_args(
                    "datetime",
                    "Format string too long (max 100 characters)",
                ));
            }
        }

        Ok(())
    }

    async fn execute(&self, args: Value) -> Result<Value, ToolError> {
        let tz = args
            .get("timezone")
            .and_then(|v| v.as_str())
            .unwrap_or("utc");

        let fmt = args
            .get("format")
            .and_then(|v| v.as_str())
            .unwrap_or("%Y-%m-%d %H:%M:%S");

        let (formatted, timezone, unix_timestamp) = match tz {
            "local" => {
                let now = Local::now();
                (
                    now.format(fmt).to_string(),
                    "local",
                    now.timestamp(),
                )
            }
            _ => {
                let now = Utc::now();
                (
                    now.format(fmt).to_string(),
                    "utc",
                    now.timestamp(),
                )
            }
        };

        Ok(serde_json::json!({
            "datetime": formatted,
            "timezone": timezone,
            "format": fmt,
            "unix_timestamp": unix_timestamp
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_utc_datetime() {
        let dt = DateTimeTool::new();
        let result = dt
            .execute(serde_json::json!({"timezone": "utc"}))
            .await
            .unwrap();

        assert_eq!(result["timezone"], "utc");
        assert!(result["datetime"].is_string());
        assert!(result["unix_timestamp"].is_i64());
    }

    #[tokio::test]
    async fn test_local_datetime() {
        let dt = DateTimeTool::new();
        let result = dt
            .execute(serde_json::json!({"timezone": "local"}))
            .await
            .unwrap();

        assert_eq!(result["timezone"], "local");
    }

    #[tokio::test]
    async fn test_custom_format() {
        let dt = DateTimeTool::new();
        let result = dt
            .execute(serde_json::json!({"format": "%Y-%m-%d"}))
            .await
            .unwrap();

        // Should be date only (YYYY-MM-DD format)
        let datetime = result["datetime"].as_str().unwrap();
        assert_eq!(datetime.len(), 10); // "2025-12-18" = 10 chars
    }

    #[tokio::test]
    async fn test_default_values() {
        let dt = DateTimeTool::new();
        let result = dt.execute(serde_json::json!({})).await.unwrap();

        assert_eq!(result["timezone"], "utc");
        assert_eq!(result["format"], "%Y-%m-%d %H:%M:%S");
    }

    #[tokio::test]
    async fn test_invalid_timezone() {
        let dt = DateTimeTool::new();
        let result = dt.validate(&serde_json::json!({"timezone": "invalid"}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }

    #[tokio::test]
    async fn test_format_too_long() {
        let dt = DateTimeTool::new();
        let long_format = "a".repeat(150);
        let result = dt.validate(&serde_json::json!({"format": long_format}));

        assert!(matches!(result, Err(ToolError::InvalidArguments { .. })));
    }
}
