use crate::id::AttestAgent;
use crate::persist::audit::{ActorType, AuditEventType, AuditStore};
use notify::{Event, EventKind, RecursiveMode, Watcher};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct AttestWatcher {
    agent: Arc<AttestAgent>,
    store: Arc<Mutex<AuditStore>>,
}

impl AttestWatcher {
    pub fn new(agent: Arc<AttestAgent>, store: Arc<Mutex<AuditStore>>) -> Self {
        Self { agent, store }
    }

    /// Watch a directory recursively and log all events as signed attestations.
    pub async fn watch<P: AsRef<Path>>(&self, path: P) -> anyhow::Result<()> {
        let (tx, mut rx) = tokio::sync::mpsc::channel(100);

        // Notify uses a sync callback, so we bridge to async mpsc
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(event) = res {
                let _ = tx.blocking_send(event);
            }
        })?;

        watcher.watch(path.as_ref(), RecursiveMode::Recursive)?;

        tracing::info!("Watcher active on: {:?}", path.as_ref());

        while let Some(event) = rx.recv().await {
            self.handle_event(event).await?;
        }

        Ok(())
    }

    async fn handle_event(&self, event: Event) -> anyhow::Result<()> {
        let paths: Vec<String> = event
            .paths
            .iter()
            .map(|p| p.to_string_lossy().to_string())
            .collect();

        let event_name = match event.kind {
            EventKind::Create(_) => "FileCreated",
            EventKind::Modify(_) => "FileModified",
            EventKind::Remove(_) => "FileDeleted",
            _ => return Ok(()), // Ignore noise
        };

        // Sign the event as a claim
        let claim_data = format!("{}:{}", event_name, paths.join(","));
        let signed_claim = self.agent.sign_claim(claim_data);

        // Persist to audit store
        let store = self.store.lock().await;
        store
            .log(
                AuditEventType::Custom(event_name.to_string()),
                Some(self.agent.to_vex_uuid()),
                serde_json::json!({
                    "paths": paths,
                    "signature": signed_claim.signature,
                    "claim_hash": signed_claim.claim.data,
                }),
                ActorType::Bot(self.agent.to_vex_uuid()),
                &self.agent,
            )
            .await?;

        Ok(())
    }
}
