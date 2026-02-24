//! API Key storage and validation
//!
//! Provides database-backed API key management with hashing and lookup.

use argon2::password_hash::{rand_core::OsRng, SaltString};
use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use subtle::ConstantTimeEq;
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
    /// Argon2id hash of the API key (includes salt, never store plaintext)
    pub key_hash: String,
    /// First 12 characters of key for identification (safe to show)
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

        // Generate a secure random key: vex_<uuid>_<random> (2025 best practice)
        use rand::distributions::{Alphanumeric, DistString};
        let random_part = Alphanumeric.sample_string(&mut rand::thread_rng(), 32);
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

    /// Hash an API key for secure storage using Argon2id with random salt
    /// Returns the PHC-formatted hash string (includes salt)
    pub fn hash_key(key: &str) -> String {
        let salt = SaltString::generate(&mut OsRng);
        let argon2 = Argon2::default();
        argon2
            .hash_password(key.as_bytes(), &salt)
            .expect("Argon2 hashing should not fail")
            .to_string()
    }

    /// Verify a plaintext key against a stored Argon2id hash
    /// Uses constant-time comparison to prevent timing attacks
    pub fn verify_key(plaintext_key: &str, stored_hash: &str) -> bool {
        match PasswordHash::new(stored_hash) {
            Ok(parsed_hash) => Argon2::default()
                .verify_password(plaintext_key.as_bytes(), &parsed_hash)
                .is_ok(),
            Err(_) => {
                // Legacy SHA-256 hash fallback (for migration)
                // Use constant-time comparison
                let legacy_hash = {
                    use sha2::{Digest, Sha256};
                    let mut hasher = Sha256::new();
                    hasher.update(plaintext_key.as_bytes());
                    hex::encode(hasher.finalize())
                };
                legacy_hash.as_bytes().ct_eq(stored_hash.as_bytes()).into()
            }
        }
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

    /// Find a key by its hash (for legacy SHA-256 compatibility)
    async fn find_by_hash(&self, hash: &str) -> Result<Option<ApiKeyRecord>, ApiKeyError>;

    /// Find and verify a key using Argon2
    /// Recommended approach: extract key ID from prefix for O(1) lookup
    async fn find_and_verify_key(
        &self,
        plaintext_key: &str,
    ) -> Result<Option<ApiKeyRecord>, ApiKeyError>;

    /// Find all keys for a user
    async fn find_by_user(&self, user_id: &str) -> Result<Vec<ApiKeyRecord>, ApiKeyError>;

    /// Update last_used_at timestamp
    async fn record_usage(&self, id: Uuid) -> Result<(), ApiKeyError>;

    /// Revoke a key
    async fn revoke(&self, id: Uuid) -> Result<(), ApiKeyError>;

    /// Delete a key
    async fn delete(&self, id: Uuid) -> Result<(), ApiKeyError>;

    /// Rotate a key: creates a new key and revokes the old one
    ///
    /// # Arguments
    /// * `old_key_id` - The ID of the key to rotate
    /// * `expires_in_days` - TTL for the new key (default: 90 days)
    ///
    /// # Returns
    /// * `(new_record, plaintext_key)` - The new record and plaintext key (shown once!)
    async fn rotate(
        &self,
        old_key_id: Uuid,
        expires_in_days: Option<u32>,
    ) -> Result<(ApiKeyRecord, String), ApiKeyError>;
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
        // For legacy SHA-256 hashes, direct comparison is still possible
        let keys = self.keys.read().await;
        Ok(keys.values().find(|r| r.key_hash == hash).cloned())
    }

    async fn find_and_verify_key(
        &self,
        plaintext_key: &str,
    ) -> Result<Option<ApiKeyRecord>, ApiKeyError> {
        // Extract ID from key: vex_<uuid_compact>_<random>
        let parts: Vec<&str> = plaintext_key.split('_').collect();
        if parts.len() < 3 {
            return Err(ApiKeyError::InvalidFormat);
        }

        let uuid_str = parts[1];
        if uuid_str.len() != 32 {
            return Err(ApiKeyError::InvalidFormat);
        }

        // Reconstruct UUID (with hyphens for parsing)
        let formatted_uuid = format!(
            "{}-{}-{}-{}-{}",
            &uuid_str[0..8],
            &uuid_str[8..12],
            &uuid_str[12..16],
            &uuid_str[16..20],
            &uuid_str[20..32]
        );

        let id = Uuid::parse_str(&formatted_uuid).map_err(|_| ApiKeyError::InvalidFormat)?;

        let keys = self.keys.read().await;
        if let Some(record) = keys.get(&id) {
            if ApiKeyRecord::verify_key(plaintext_key, &record.key_hash) {
                return Ok(Some(record.clone()));
            }
        }
        
        Ok(None)
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

    async fn rotate(
        &self,
        old_key_id: Uuid,
        expires_in_days: Option<u32>,
    ) -> Result<(ApiKeyRecord, String), ApiKeyError> {
        // Get old key to copy user_id, name, and scopes
        let old_key = {
            let keys = self.keys.read().await;
            keys.get(&old_key_id)
                .cloned()
                .ok_or(ApiKeyError::NotFound)?
        };

        // Revoke old key first
        self.revoke(old_key_id).await?;

        // Create new key with same user, name suffix, and scopes
        let ttl = expires_in_days.unwrap_or(90); // Default 90 days per 2025 best practices
        let (new_record, plaintext) = ApiKeyRecord::new(
            &old_key.user_id,
            &format!("{} (rotated)", old_key.name),
            old_key.scopes.clone(),
            Some(ttl),
        );

        // Store new key
        self.create(&new_record).await?;

        tracing::info!(
            old_key_id = %old_key_id,
            new_key_id = %new_record.id,
            user_id = %old_key.user_id,
            "API key rotated successfully"
        );

        Ok((new_record, plaintext))
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

    // Find and verify using Argon2 (handles both new and legacy hashes)
    let record = store
        .find_and_verify_key(plaintext_key)
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
    async fn test_api_key_hash_verification() {
        // With Argon2id, hashes are different each time (due to random salt)
        // Instead, we test that verify_key correctly validates
        let key = "vex_test123456789_abcdefghijklmnopqrst";
        let hash = ApiKeyRecord::hash_key(key);

        // Same key should verify against its hash
        assert!(ApiKeyRecord::verify_key(key, &hash));

        // Different key should not verify
        assert!(!ApiKeyRecord::verify_key(
            "vex_wrong_key_12345678901234567890",
            &hash
        ));
    }

    #[tokio::test]
    async fn test_memory_store_crud() {
        let store = MemoryApiKeyStore::new();
        let (record, key) = ApiKeyRecord::new("user1", "Test Key", vec![], None);

        // Create
        store.create(&record).await.unwrap();

        // Find and verify key (uses Argon2 verification)
        let found = store.find_and_verify_key(&key).await.unwrap();
        assert!(found.is_some());
        assert_eq!(found.unwrap().id, record.id);

        // Revoke
        store.revoke(record.id).await.unwrap();
        let revoked = store.find_and_verify_key(&key).await.unwrap().unwrap();
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
