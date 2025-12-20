//! A2A Task types
//!
//! Types for A2A task requests and responses.
//!
//! # Security
//!
//! - Nonce + timestamp for replay protection
//! - Task IDs are UUIDs (unguessable)
//! - Responses include Merkle hash for verification

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// A2A Task request from another agent
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskRequest {
    /// Unique task ID (created by caller or generated)
    #[serde(default = "Uuid::new_v4")]
    pub id: Uuid,
    /// Skill ID that the agent should use
    pub skill: String,
    /// Input data for the task
    pub input: serde_json::Value,
    /// Calling agent's identifier
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_agent: Option<String>,
    /// Nonce for replay protection
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nonce: Option<String>,
    /// Request timestamp
    #[serde(default = "Utc::now")]
    pub timestamp: DateTime<Utc>,
}

/// A2A Task response
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct TaskResponse {
    /// Task ID (matches request)
    pub id: Uuid,
    /// Current status
    pub status: TaskStatus,
    /// Result data (if completed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    /// Error message (if failed)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Merkle hash of the result for verification
    #[serde(skip_serializing_if = "Option::is_none")]
    pub merkle_hash: Option<String>,
    /// Response timestamp
    pub timestamp: DateTime<Utc>,
}

/// Task execution status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, utoipa::ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TaskStatus {
    /// Task is queued
    Pending,
    /// Task is running
    Running,
    /// Task completed successfully
    Completed,
    /// Task failed
    Failed,
    /// Task was cancelled
    Cancelled,
}

impl TaskRequest {
    /// Create a new task request
    pub fn new(skill: impl Into<String>, input: serde_json::Value) -> Self {
        Self {
            id: Uuid::new_v4(),
            skill: skill.into(),
            input,
            caller_agent: None,
            nonce: None,
            timestamp: Utc::now(),
        }
    }

    /// Add caller agent info
    pub fn with_caller(mut self, agent: impl Into<String>) -> Self {
        self.caller_agent = Some(agent.into());
        self
    }

    /// Add nonce for replay protection
    pub fn with_nonce(mut self, nonce: impl Into<String>) -> Self {
        self.nonce = Some(nonce.into());
        self
    }
}

impl TaskResponse {
    /// Create a pending response
    pub fn pending(id: Uuid) -> Self {
        Self {
            id,
            status: TaskStatus::Pending,
            result: None,
            error: None,
            merkle_hash: None,
            timestamp: Utc::now(),
        }
    }

    /// Create a completed response
    pub fn completed(id: Uuid, result: serde_json::Value, merkle_hash: impl Into<String>) -> Self {
        Self {
            id,
            status: TaskStatus::Completed,
            result: Some(result),
            error: None,
            merkle_hash: Some(merkle_hash.into()),
            timestamp: Utc::now(),
        }
    }

    /// Create a failed response
    pub fn failed(id: Uuid, error: impl Into<String>) -> Self {
        Self {
            id,
            status: TaskStatus::Failed,
            result: None,
            error: Some(error.into()),
            merkle_hash: None,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_task_request_new() {
        let req = TaskRequest::new("verify", serde_json::json!({"claim": "test"}));
        assert_eq!(req.skill, "verify");
        assert!(req.caller_agent.is_none());
    }

    #[test]
    fn test_task_request_builder() {
        let req = TaskRequest::new("verify", serde_json::json!({}))
            .with_caller("other-agent")
            .with_nonce("abc123");

        assert_eq!(req.caller_agent, Some("other-agent".to_string()));
        assert_eq!(req.nonce, Some("abc123".to_string()));
    }

    #[test]
    fn test_task_response_completed() {
        let id = Uuid::new_v4();
        let resp = TaskResponse::completed(id, serde_json::json!({"verified": true}), "hash123");

        assert_eq!(resp.id, id);
        assert_eq!(resp.status, TaskStatus::Completed);
        assert!(resp.result.is_some());
        assert_eq!(resp.merkle_hash, Some("hash123".to_string()));
    }

    #[test]
    fn test_task_response_failed() {
        let id = Uuid::new_v4();
        let resp = TaskResponse::failed(id, "Verification failed");

        assert_eq!(resp.status, TaskStatus::Failed);
        assert_eq!(resp.error, Some("Verification failed".to_string()));
    }

    #[test]
    fn test_serialization() {
        let req = TaskRequest::new("hash", serde_json::json!({"text": "hello"}));
        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("hash"));

        let resp = TaskResponse::pending(req.id);
        let json = serde_json::to_string(&resp).unwrap();
        assert!(json.contains("pending"));
    }
}
