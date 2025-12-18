//! Streaming tool support for long-running operations
//!
//! This module provides traits and types for tools that produce streaming output.
//!
//! # Security Considerations
//!
//! - **Backpressure**: Streams MUST respect consumer pace (DoS prevention)
//! - **Timeouts**: Per-chunk timeouts in addition to total timeout
//! - **Resource Limits**: Maximum chunks per stream to prevent memory exhaustion
//! - **Cancellation**: Streams support graceful cancellation via Drop
//!
//! # Example
//!
//! ```ignore
//! use vex_llm::streaming_tool::{StreamingTool, ToolChunk};
//!
//! let stream = tool.execute_stream(args);
//! pin_mut!(stream);
//! while let Some(chunk) = stream.next().await {
//!     match chunk {
//!         ToolChunk::Progress { percent, message } => println!("{}% - {}", percent, message),
//!         ToolChunk::Complete { result } => println!("Done: {:?}", result.hash),
//!         _ => {}
//!     }
//! }
//! ```

use std::pin::Pin;
use std::time::Duration;

use async_trait::async_trait;
use futures::Stream;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::tool::Tool;
use crate::tool_error::ToolError;
use crate::tool_result::ToolResult;

/// A chunk of streaming output from a tool.
///
/// # Security
///
/// - Progress updates are rate-limited by design (max 1/100ms recommended)
/// - Partial data is NOT hashed until Complete (prevents hash oracle attacks)
/// - Errors stop the stream immediately
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolChunk {
    /// Progress update (percentage and message)
    Progress {
        /// Progress percentage (0.0 to 100.0)
        percent: f32,
        /// Human-readable status message
        message: String,
    },

    /// Partial data chunk (intermediate result)
    Partial {
        /// Partial data (NOT hashed for security)
        data: Value,
        /// Chunk index (0-based)
        index: usize,
    },

    /// Final complete result with cryptographic hash
    Complete {
        /// Final result with Merkle-compatible hash
        result: ToolResult,
    },

    /// Error during streaming (terminates stream)
    /// Note: Uses String to allow Clone/Serialize without ToolError constraints
    Error {
        /// Tool name that failed
        tool: String,
        /// Error message (sanitized)
        message: String,
        /// Whether the error is retryable
        retryable: bool,
    },
}

impl ToolChunk {
    /// Create a progress chunk
    pub fn progress(percent: f32, message: impl Into<String>) -> Self {
        Self::Progress {
            percent: percent.clamp(0.0, 100.0),
            message: message.into(),
        }
    }

    /// Create a partial data chunk
    pub fn partial(data: Value, index: usize) -> Self {
        Self::Partial { data, index }
    }

    /// Create a complete chunk from a tool result
    pub fn complete(result: ToolResult) -> Self {
        Self::Complete { result }
    }

    /// Create an error chunk from a ToolError
    pub fn from_error(error: &ToolError) -> Self {
        Self::Error {
            tool: match error {
                ToolError::NotFound { name } => name.clone(),
                ToolError::InvalidArguments { tool, .. } => tool.clone(),
                ToolError::ExecutionFailed { tool, .. } => tool.clone(),
                ToolError::Timeout { tool, .. } => tool.clone(),
                ToolError::Unavailable { name, .. } => name.clone(),
                ToolError::Serialization(_) => "serialization".to_string(),
                ToolError::AuditFailed(_) => "audit".to_string(),
            },
            message: error.to_string(),
            retryable: error.is_retryable(),
        }
    }

    /// Create a simple error chunk
    pub fn error(tool: impl Into<String>, message: impl Into<String>) -> Self {
        Self::Error {
            tool: tool.into(),
            message: message.into(),
            retryable: false,
        }
    }

    /// Check if this chunk terminates the stream
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Complete { .. } | Self::Error { .. })
    }
}

/// Type alias for a boxed async stream of tool chunks
pub type ToolStream = Pin<Box<dyn Stream<Item = ToolChunk> + Send>>;

/// Configuration for streaming tool execution
///
/// # Security
///
/// - `max_chunks`: Prevents unbounded memory growth (DoS)
/// - `chunk_timeout`: Prevents hanging streams (DoS)
/// - `max_duration`: Total execution limit
#[derive(Debug, Clone)]
pub struct StreamConfig {
    /// Maximum number of chunks before forced termination
    pub max_chunks: usize,
    /// Timeout for each individual chunk
    pub chunk_timeout: Duration,
    /// Maximum total duration for the stream
    pub max_duration: Duration,
    /// Minimum interval between progress updates (rate limiting)
    pub min_progress_interval: Duration,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            max_chunks: 1000,
            chunk_timeout: Duration::from_secs(30),
            max_duration: Duration::from_secs(300), // 5 minutes
            min_progress_interval: Duration::from_millis(100),
        }
    }
}

impl StreamConfig {
    /// Create config for short operations
    pub fn short() -> Self {
        Self {
            max_chunks: 100,
            chunk_timeout: Duration::from_secs(5),
            max_duration: Duration::from_secs(30),
            min_progress_interval: Duration::from_millis(50),
        }
    }

    /// Create config for long operations
    pub fn long() -> Self {
        Self {
            max_chunks: 10000,
            chunk_timeout: Duration::from_secs(60),
            max_duration: Duration::from_secs(3600), // 1 hour
            min_progress_interval: Duration::from_millis(500),
        }
    }
}

/// Trait for tools that produce streaming output.
///
/// # Security
///
/// Implementors MUST:
/// - Respect cancellation (check for stream drop)
/// - Limit output size (respect StreamConfig)
/// - Hash only the final result (not intermediate chunks)
/// - Sanitize all output data
#[async_trait]
pub trait StreamingTool: Tool {
    /// Execute with streaming output
    ///
    /// Returns a stream of `ToolChunk` values. The stream MUST:
    /// - Emit at least one `Complete` or `Error` chunk before ending
    /// - Respect the provided configuration limits
    /// - Be cancellable (stop when dropped)
    fn execute_stream(&self, args: Value, config: StreamConfig) -> ToolStream;

    /// Get the default stream configuration for this tool
    fn stream_config(&self) -> StreamConfig {
        StreamConfig::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_tool_chunk_progress() {
        let chunk = ToolChunk::progress(50.0, "Halfway there");
        match chunk {
            ToolChunk::Progress { percent, message } => {
                assert_eq!(percent, 50.0);
                assert_eq!(message, "Halfway there");
            }
            _ => panic!("Expected Progress chunk"),
        }
    }

    #[test]
    fn test_tool_chunk_progress_clamped() {
        let chunk = ToolChunk::progress(150.0, "Over 100");
        match chunk {
            ToolChunk::Progress { percent, .. } => {
                assert_eq!(percent, 100.0);
            }
            _ => panic!("Expected Progress chunk"),
        }
    }

    #[test]
    fn test_tool_chunk_is_terminal() {
        assert!(!ToolChunk::progress(50.0, "").is_terminal());
        assert!(!ToolChunk::partial(serde_json::json!({}), 0).is_terminal());
        
        let result = ToolResult::new("test", &serde_json::json!({}), serde_json::json!({}), Duration::from_secs(1));
        assert!(ToolChunk::complete(result).is_terminal());
        
        assert!(ToolChunk::error("test", "not found").is_terminal());
    }

    #[test]
    fn test_stream_config_default() {
        let config = StreamConfig::default();
        assert_eq!(config.max_chunks, 1000);
        assert_eq!(config.max_duration, Duration::from_secs(300));
    }

    #[test]
    fn test_stream_config_short() {
        let config = StreamConfig::short();
        assert_eq!(config.max_chunks, 100);
        assert!(config.max_duration < Duration::from_secs(60));
    }

    #[test]
    fn test_stream_config_long() {
        let config = StreamConfig::long();
        assert_eq!(config.max_chunks, 10000);
        assert_eq!(config.max_duration, Duration::from_secs(3600));
    }
}
