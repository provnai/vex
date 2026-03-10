use crate::error::AttestError;
use crate::persist::sqlite::LocalStore;
use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditEventType {
    AgentCreated,
    AgentExecuted,
    FileSystemChange,
    ProcessExecution,
    NetworkInteraction,
    PolicyUpdate,
    PolicyViolated,
    Custom(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActorType {
    System,
    Agent(String),
    User(String),
    Bot(uuid::Uuid),
}

impl ActorType {
    pub fn pseudonymize(&self) -> Self {
        match self {
            Self::System => Self::System,
            Self::Agent(id) => Self::Agent(format!("aid:{}", &id[..8])),
            Self::User(name) => Self::User(format!("user:{}", &name[..3])),
            Self::Bot(id) => Self::Bot(*id),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: uuid::Uuid,
    pub event_type: AuditEventType,
    pub timestamp: DateTime<Utc>,
    pub agent_id: Option<uuid::Uuid>,
    pub data: serde_json::Value,
    pub hash: String,
    pub previous_hash: Option<String>,
    pub sequence_number: u64,
    pub actor: ActorType,
    pub rationale: Option<String>,
    pub signature: Option<String>,
    pub zk_proof: Option<String>,
}

impl AuditEvent {
    pub fn compute_hash(&self) -> String {
        use sha2::{Digest, Sha256};
        let mut hasher = Sha256::new();
        hasher.update(self.id.as_bytes());
        hasher.update(serde_json::to_string(&self.event_type).unwrap_or_default());
        hasher.update(self.timestamp.timestamp().to_le_bytes());
        hasher.update(serde_json::to_string(&self.data).unwrap_or_default());
        if let Some(prev) = &self.previous_hash {
            hasher.update(prev.as_bytes());
        }
        hasher.update(self.sequence_number.to_le_bytes());
        hasher.update(serde_json::to_string(&self.actor).unwrap_or_default());

        hex::encode(hasher.finalize())
    }

    pub fn chain(mut self, previous_hash: String) -> Self {
        self.previous_hash = Some(previous_hash);
        self.hash = self.compute_hash();
        self
    }
}

pub struct AuditStore {
    pub local: Arc<LocalStore>,
}

impl AuditStore {
    pub fn new(local: Arc<LocalStore>) -> Self {
        Self { local }
    }

    pub async fn log(
        &self,
        event_type: AuditEventType,
        agent_id: Option<uuid::Uuid>,
        data: serde_json::Value,
        actor: ActorType,
        agent: &crate::id::AttestAgent,
    ) -> Result<AuditEvent, AttestError> {
        let last_hash = self.local.get_last_hash().await?;
        let sequence = self.local.get_next_sequence().await?;

        let mut event = AuditEvent {
            id: uuid::Uuid::new_v4(),
            event_type,
            timestamp: Utc::now(),
            agent_id,
            data,
            hash: String::new(),
            previous_hash: None,
            sequence_number: sequence,
            actor: actor.pseudonymize(),
            rationale: None,
            signature: None,
            zk_proof: None,
        };

        // 1. Compute Hash (Chain)
        if let Some(ref prev) = last_hash {
            event = event.chain(prev.clone());
        } else {
            event.hash = event.compute_hash();
        }

        // 2. Sign the Hash with Agent Identity
        use ed25519_dalek::Signer;
        let signature = agent.signing_key.sign(event.hash.as_bytes());
        event.signature = Some(hex::encode(signature.to_bytes()));

        self.local.save_event(&event).await?;

        // 3. Generate ZK Proof
        let prev_root = last_hash
            .as_ref()
            .map(|h| {
                let mut arr = [0u8; 32];
                let decoded = hex::decode(h).unwrap_or_default();
                let len = decoded.len().min(32);
                arr[..len].copy_from_slice(&decoded[..len]);
                arr
            })
            .unwrap_or([0u8; 32]);

        let event_hash_bytes = hex::decode(&event.hash).unwrap_or_default();
        let mut event_hash = [0u8; 32];
        let hash_len = event_hash_bytes.len().min(32);
        event_hash[..hash_len].copy_from_slice(&event_hash_bytes[..hash_len]);

        if let Ok(proof) = crate::zk::AuditProver::prove_transition(prev_root, event_hash) {
            let mut updated_event = event.clone();
            updated_event.zk_proof = Some(hex::encode(proof));
            self.local.save_event(&updated_event).await?;
            return Ok(updated_event);
        }

        Ok(event)
    }

    /// Verify the ZK proof integrity for the entire chain.
    pub async fn verify_zk_integrity(&self) -> Result<bool, AttestError> {
        let events = self.local.get_all_events().await?;
        if events.is_empty() {
            return Ok(true);
        }

        let initial_root_hex = events[0].previous_hash.clone().unwrap_or_else(|| {
            "0000000000000000000000000000000000000000000000000000000000000000".into()
        });
        let initial_root = hex::decode(&initial_root_hex)
            .map_err(|_| AttestError::Alignment("Invalid initial root hex".into()))?;
        let mut initial_arr = [0u8; 32];
        initial_arr.copy_from_slice(&initial_root[..32]);

        for event in events {
            if let Some(proof_hex) = &event.zk_proof {
                let proof = hex::decode(proof_hex)
                    .map_err(|_| AttestError::Alignment("Invalid proof hex".into()))?;
                let mut final_arr = [0u8; 32];
                let decoded_final = hex::decode(&event.hash).unwrap_or_default();
                if decoded_final.len() >= 32 {
                    final_arr.copy_from_slice(&decoded_final[..32]);
                }

                if !crate::zk::AuditProver::verify_proof(&proof, initial_arr, final_arr)
                    .unwrap_or(false)
                {
                    tracing::error!(
                        "ZK Proof verification failed at sequence {}",
                        event.sequence_number
                    );
                    return Ok(false);
                }
                initial_arr = final_arr;
            }
        }
        Ok(true)
    }

    /// Verify the cryptographic integrity of the entire audit chain and all signatures.
    pub async fn verify_integrity(&self) -> Result<bool, AttestError> {
        let events = self.local.get_all_events().await?;
        let mut expected_prev_hash: Option<String> = None;

        for (i, event) in events.iter().enumerate() {
            if event.sequence_number != i as u64 {
                return Ok(false);
            }
            if event.previous_hash != expected_prev_hash {
                return Ok(false);
            }
            if event.compute_hash() != event.hash {
                return Ok(false);
            }
            expected_prev_hash = Some(event.hash.clone());
        }
        Ok(true)
    }

    /// Verify integrity with a specific agent's public key
    pub async fn verify_integrity_with_key(
        &self,
        verifying_key: &ed25519_dalek::VerifyingKey,
    ) -> Result<bool, AttestError> {
        let events = self.local.get_all_events().await?;
        let mut expected_prev_hash: Option<String> = None;

        for (i, event) in events.iter().enumerate() {
            if event.sequence_number != i as u64 {
                return Ok(false);
            }
            if event.previous_hash != expected_prev_hash {
                return Ok(false);
            }
            if event.compute_hash() != event.hash {
                return Ok(false);
            }

            if let Some(sig_hex) = &event.signature {
                let sig_bytes = hex::decode(sig_hex)
                    .map_err(|_| AttestError::Crypto("Invalid signature hex".into()))?;
                let signature = ed25519_dalek::Signature::from_slice(&sig_bytes)
                    .map_err(|_| AttestError::Crypto("Invalid signature format".into()))?;

                use ed25519_dalek::Verifier;
                if verifying_key
                    .verify(event.hash.as_bytes(), &signature)
                    .is_err()
                {
                    tracing::error!(
                        "Signature verification failed at seq {}",
                        event.sequence_number
                    );
                    return Ok(false);
                }
            } else {
                tracing::error!("Missing signature at seq {}", event.sequence_number);
                return Ok(false);
            }

            expected_prev_hash = Some(event.hash.clone());
        }
        Ok(true)
    }
}
