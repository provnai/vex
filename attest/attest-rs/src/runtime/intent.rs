use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum IntentStatus {
    Open,
    InProgress,
    Completed,
    Failed,
    Canceled,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Intent {
    pub id: String,
    pub goal: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(rename = "ticketId", skip_serializing_if = "Option::is_none")]
    pub ticket_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub constraints: Vec<String>,
    #[serde(
        rename = "acceptanceCriteria",
        skip_serializing_if = "Vec::is_empty",
        default
    )]
    pub acceptance_criteria: Vec<String>,
    pub status: String,
    #[serde(rename = "createdAt")]
    pub created_at: String,
    #[serde(rename = "closedAt", skip_serializing_if = "Option::is_none")]
    pub closed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<IntentMeta>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IntentMeta {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub assignee: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub epic: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub labels: Vec<String>,
    #[serde(
        rename = "customData",
        skip_serializing_if = "HashMap::is_empty",
        default
    )]
    pub custom_data: HashMap<String, String>,
}

impl Intent {
    pub fn new(id: String, goal: String) -> Self {
        Self {
            id,
            goal,
            description: None,
            ticket_id: None,
            constraints: Vec::new(),
            acceptance_criteria: Vec::new(),
            status: "open".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            closed_at: None,
            metadata: None,
        }
    }
}
