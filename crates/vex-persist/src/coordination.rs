//! # Coordination Ledger
//!
//! Provides stateful tracking for AI escalations and resolutions.
//! Links disparate audit events into a single "Coordination State Machine".

use crate::backend::{StorageBackend, StorageError, StorageExt};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use uuid::Uuid;

/// The state of an AI Escalation
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CoordinationStatus {
    /// Escalation has been recorded but no resolution yet.
    Escalated,
    /// A human reviewer has resolved the escalation.
    Resolved,
    /// Escalation was rejected or expired.
    Expired,
}

/// A Coordination Record binds an Escalation event to its eventual Resolution.
#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CoordinationRecord {
    pub escalation_id: String,
    pub status: CoordinationStatus,
    pub escalation_event_id: Uuid,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_event_id: Option<Uuid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub resolution_vep_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub continuation_token: Option<vex_core::ContinuationToken>,
    pub timestamp: chrono::DateTime<chrono::Utc>,
}

/// Interface for the Coordination Ledger
#[async_trait::async_trait]
pub trait CoordinationStore: Send + Sync {
    /// Record a new escalation
    async fn record_escalation(
        &self,
        tenant_id: &str,
        escalation_id: String,
        event_id: Uuid,
        continuation_token: Option<vex_core::ContinuationToken>,
    ) -> Result<(), StorageError>;

    /// Resolve an existing escalation
    async fn resolve_escalation(
        &self,
        tenant_id: &str,
        escalation_id: &str,
        resolution_event_id: Uuid,
        resolution_vep_hash: String,
    ) -> Result<(), StorageError>;

    /// Get the current status of an escalation
    async fn get_record(
        &self,
        tenant_id: &str,
        escalation_id: &str,
    ) -> Result<Option<CoordinationRecord>, StorageError>;

    /// List all active escalations for a tenant
    async fn list_active(&self, tenant_id: &str) -> Result<Vec<CoordinationRecord>, StorageError>;
}

/// Default implementation of CoordinationStore using the generic StorageBackend.
pub struct PersistentCoordinationStore<B: StorageBackend + ?Sized> {
    backend: Arc<B>,
    prefix: String,
}

impl<B: StorageBackend + ?Sized> PersistentCoordinationStore<B> {
    pub fn new(backend: Arc<B>) -> Self {
        Self {
            backend,
            prefix: "coordination:".to_string(),
        }
    }

    fn record_key(&self, tenant_id: &str, escalation_id: &str) -> String {
        format!(
            "{}tenant:{}:record:{}",
            self.prefix, tenant_id, escalation_id
        )
    }

    fn active_list_key(&self, tenant_id: &str) -> String {
        format!("{}tenant:{}:active", self.prefix, tenant_id)
    }
}

#[async_trait::async_trait]
impl<B: StorageBackend + ?Sized> CoordinationStore for PersistentCoordinationStore<B> {
    async fn record_escalation(
        &self,
        tenant_id: &str,
        escalation_id: String,
        event_id: Uuid,
        continuation_token: Option<vex_core::ContinuationToken>,
    ) -> Result<(), StorageError> {
        let record = CoordinationRecord {
            escalation_id: escalation_id.clone(),
            status: CoordinationStatus::Escalated,
            escalation_event_id: event_id,
            resolution_event_id: None,
            resolution_vep_hash: None,
            continuation_token,
            timestamp: chrono::Utc::now(),
        };

        // Save record
        let key = self.record_key(tenant_id, &escalation_id);
        self.backend.set(&key, &record).await?;

        // Add to active list
        let mut active: Vec<String> = self
            .backend
            .get(&self.active_list_key(tenant_id))
            .await?
            .unwrap_or_default();

        if !active.contains(&escalation_id) {
            active.push(escalation_id);
            self.backend
                .set(&self.active_list_key(tenant_id), &active)
                .await?;
        }

        Ok(())
    }

    async fn resolve_escalation(
        &self,
        tenant_id: &str,
        escalation_id: &str,
        resolution_event_id: Uuid,
        resolution_vep_hash: String,
    ) -> Result<(), StorageError> {
        let key = self.record_key(tenant_id, escalation_id);
        let mut record: CoordinationRecord = self.backend.get(&key).await?.ok_or_else(|| {
            StorageError::NotFound(format!("Escalation {} not found", escalation_id))
        })?;

        record.status = CoordinationStatus::Resolved;
        record.resolution_event_id = Some(resolution_event_id);
        record.resolution_vep_hash = Some(resolution_vep_hash);
        record.timestamp = chrono::Utc::now();

        // Update record
        self.backend.set(&key, &record).await?;

        // Remove from active list
        let mut active: Vec<String> = self
            .backend
            .get(&self.active_list_key(tenant_id))
            .await?
            .unwrap_or_default();

        active.retain(|id| id != escalation_id);
        self.backend
            .set(&self.active_list_key(tenant_id), &active)
            .await?;

        Ok(())
    }

    async fn get_record(
        &self,
        tenant_id: &str,
        escalation_id: &str,
    ) -> Result<Option<CoordinationRecord>, StorageError> {
        self.backend
            .get(&self.record_key(tenant_id, escalation_id))
            .await
    }

    async fn list_active(&self, tenant_id: &str) -> Result<Vec<CoordinationRecord>, StorageError> {
        let active_ids: Vec<String> = self
            .backend
            .get(&self.active_list_key(tenant_id))
            .await?
            .unwrap_or_default();

        let mut out = Vec::new();
        for id in active_ids {
            if let Some(record) = self.get_record(tenant_id, &id).await? {
                out.push(record);
            }
        }
        Ok(out)
    }
}
