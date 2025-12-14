//! API Key storage and validation
//!
//! Provides database-backed API key management with hashing and lookup.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;
use uuid::Uuid;

/// API key errors
#[derive(Debug, Error)]
pub enum ApiKeyError {
    #[error("Storage error: {0}")]
    Storage(String),
    #[error("Key not found")]
    NotFound,
    #[error("Key expired")]
    Expired,
    #[error("Key revoked")]
    Revoked,
    #[error("Invalid key format")]
    InvalidFormat,
}

/// An API key record stored in the database
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ApiKeyRecord {
    /// Unique key ID (not the actual key)
    pub id: Uuid,
    /// Hash of the API key (never store plaintext)
    pub key_hash: String,
    /// First 8 characters of key for identification (safe to show)
    pub key_prefix: String,
    /// User ID this key belongs to
    pub user_id: String,
    /// Human-readable name for the key
    pub name: String,
    /// Scopes/permissions granted
    pub scopes: Vec<String>,
    /// When the key was created
    pub created_at: DateTime<Utc>,
    /// When the key expires (None = never)
    pub expires_at: Option<DateTime<Utc>>,
    /// When the key was last used
    pub last_used_at: Option<DateTime<Utc>>,
    /// Whether the key is revoked
    pub revoked: bool,
}

impl ApiKeyRecord {
    /// Create a new API key record (does not store it)
    /// Returns (record, plaintext_key) - the plaintext key is only available once!
    pub fn new(
        user_id: &str,
        name: &str,
        scopes: Vec<String>,
        expires_in_days: Option<u32>,
    ) -> (Self, String) {
        let id = Uuid::new_v4();

        // Generate a secure random key: vex_<uuid>_<random>
        let random_part: String = (0..32)
            .map(|_| {
                let idx = rand::random::<usize>() % 62;
                let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                chars.chars().nth(idx).unwrap()
            })
            .collect();
        let plaintext_key = format!("vex_{}_{}", id.to_string().replace("-", ""), random_part);

        // Hash the key for storage
        let key_hash = Self::hash_key(&plaintext_key);
        let key_prefix = plaintext_key.chars().take(12).collect();

        let expires_at =
            expires_in_days.map(|days| Utc::now() + chrono::Duration::days(days as i64));

        let record = Self {
            id,
            key_hash,
            key_prefix,
            user_id: user_id.to_string(),
            name: name.to_string(),
            scopes,
            created_at: Utc::now(),
            expires_at,
            last_used_at: None,
            revoked: false,
        };

        (record, plaintext_key)
    }

    /// Hash an API key for secure storage
    pub fn hash_key(key: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(key.as_bytes());
        hex::encode(hasher.finalize())
    }

    /// Check if this key is valid (not expired or revoked)
    pub fn is_valid(&self) -> bool {
        if self.revoked {
            return false;
        }
        if let Some(expires) = self.expires_at {
            if Utc::now() > expires {
                return false;
            }
        }
        true
    }

    /// Check if this key has a specific scope
    pub fn has_scope(&self, scope: &str) -> bool {
        self.scopes.iter().any(|s| s == scope || s == "*")
    }
}

/// API key storage trait
#[async_trait]
pub trait ApiKeyStore: Send + Sync {
    /// Store a new API key
    async fn create(&self, record: &ApiKeyRecord) -> Result<(), ApiKeyError>;

    /// Find a key by its hash
    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>, ApiKeyError>;

    /// Find all keys for a user
    async fn find_by_user(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>, ApiKeyError>;

    /// Update last_used_at timestamp
    async fn record_usage(&self, id: Uuid) -> Result<(), ApiKeyError>;

    /// Revoke a key
    async fn revoke(&self, id: Uuid) -> Result<(), ApiKeyError>;

    /// Delete a key
    async fn delete(&self, id: Uuid) -> Result<(), ApiKeyError>;
}

/// In-memory implementation of API key store (for testing)
#[derive(Debug, Default)]
pub struct MemoryApiKeyStore {
    keys: tokio::sync::RwLock<std::collections::HashMap<Uuid, ApiKeyRecord>>,
}

impl MemoryApiKeyStore {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl ApiKeyStore for MemoryApiKeyStore {
    async fn create(&self, record: &ApiKeyRecord) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        keys.insert(record.id, record.clone());
        Ok(())
    }

    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>, ApiKeyError> {
        let keys = self.keys.read().await;
        Ok(keys.values().find(|r| r.key_hash == hash).cloned())
    }

    async fn find_by_user(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>, ApiKeyError> {
        let keys = self.keys.read().await;
        Ok(keys
            .values()
            .filter(|r| r.user_id == user_id)
            .cloned()
            .collect())
    }

    async fn record_usage(&self, id: Uuid) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        if let Some(record) = keys.get_mut(&id) {
            record.last_used_at = Some(Utc::now());
            Ok(())
        } else {
            Err(ApiKeyError::NotFound)
        }
    }

    async fn revoke(&self, id: Uuid) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        if let Some(record) = keys.get_mut(&id) {
            record.revoked = true;
            Ok(())
        } else {
            Err(ApiKeyError::NotFound)
        }
    }

    async fn delete(&self, id: Uuid) -> Result<(), ApiKeyError> {
        let mut keys = self.keys.write().await;
        keys.remove(&id).ok_or(ApiKeyError::NotFound)?;
        Ok(())
    }
}

/// Validate an API key and return the associated record if valid
pub async fn validate_api_key<S: ApiKeyStore>(
    store: &S,
    plaintext_key: &str,
) -> Result<ApiKeyRecord, ApiKeyError> {
    // Validate format
    if !plaintext_key.starts_with("vex_") || plaintext_key.len() < 40 {
        return Err(ApiKeyError::InvalidFormat);
    }

    // Hash and lookup
    let hash = ApiKeyRecord::hash_key(plaintext_key);
    let record = store
        .find_by_hash(&hash)
        .await?
        .ok_or(ApiKeyError::NotFound)?;

    // Check validity
    if record.revoked {
        return Err(ApiKeyError::Revoked);
    }
    if let Some(expires) = record.expires_at {
        if Utc::now() > expires {
            return Err(ApiKeyError::Expired);
        }
    }

    // Record usage (fire-and-forget)
    let _ = store.record_usage(record.id).await;

    Ok(record)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_api_key_creation() {
        let (record, key) =
            ApiKeyRecord::new("user123", "My API Key", vec!["read".to_string()], None);

        assert!(key.starts_with("vex_"));
        assert!(key.len() > 40);
        assert_eq!(record.user_id, "user123");
        assert_eq!(record.name, "My API Key");
        assert!(record.is_valid());
    }

    #[tokio::test]
    async fn test_api_key_hash_consistency() {
        let key = "vex_test123_abcdefgh";
        let hash1 = ApiKeyRecord::hash_key(key);
        let hash2 = ApiKeyRecord::hash_key(key);
        assert_eq!(hash1, hash2);
    }

    #[tokio::test]
    async fn test_memory_store_crud() {
        let store = MemoryApiKeyStore::new();
        let (record, key) = ApiKeyRecord::new("user1", "Test Key", vec![], None);

        // Create
        store.create(&record).await.unwrap();

        // Find by hash
        let hash = ApiKeyRecord::hash_key(&key);
        let found = store.find_by_hash(&hash).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, record.id);

        // Revoke
        store.revoke(record.id).await.unwrap();
        let revoked = store.find_by_hash(&hash).await.unwrap().unwrap();
        assert!(revoked.revoked);
    }

    #[tokio::test]
    async fn test_validate_api_key() {
        let store = MemoryApiKeyStore::new();
        let (record, key) = ApiKeyRecord::new("user1", "Test Key", vec!["admin".to_string()], None);
        store.create(&record).await.unwrap();

        // Valid key should work
        let validated = validate_api_key(&store, &key).await.unwrap();
        assert_eq!(validated.id, record.id);
        assert!(validated.has_scope("admin"));

        // Invalid format should fail
        let result = validate_api_key(&store, "invalid").await;
        assert!(matches!(result, Err(ApiKeyError::InvalidFormat)));

        // Wrong key should fail
        let result =
            validate_api_key(&store, "vex_00000000000000000000000000000000_wrongkey").await;
        assert!(matches!(result, Err(ApiKeyError::NotFound)));
    }
}
