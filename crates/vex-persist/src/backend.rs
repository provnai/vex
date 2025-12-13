//! Storage backend trait and error types

use async_trait::async_trait;
use serde::{de::DeserializeOwned, Serialize};
use std::fmt::Debug;

/// Storage error types
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("Not found: {0}")]
    NotFound(String),
    
    #[error("Already exists: {0}")]
    AlreadyExists(String),
    
    #[error("Serialization error: {0}")]
    Serialization(String),
    
    #[error("Connection error: {0}")]
    Connection(String),
    
    #[error("Query error: {0}")]
    Query(String),
    
    #[error("Internal error: {0}")]
    Internal(String),
}

/// Generic storage backend trait (Object Safe)
#[async_trait]
pub trait StorageBackend: Send + Sync + Debug {
    /// Get the backend name
    fn name(&self) -> &str;
    
    /// Check if backend is healthy
    async fn is_healthy(&self) -> bool;
    
    /// Store a JSON value with a key
    async fn set_value(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError>;
    
    /// Get a JSON value by key
    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError>;
    
    /// Delete a value by key
    async fn delete(&self, key: &str) -> Result<bool, StorageError>;
    
    /// Check if key exists
    async fn exists(&self, key: &str) -> Result<bool, StorageError>;
    
    /// List all keys with prefix
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError>;
}

/// Extension trait for typed access
#[async_trait]
pub trait StorageExt {
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), StorageError>;
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StorageError>;
}

#[async_trait]
impl<S: StorageBackend + ?Sized> StorageExt for S {
    async fn set<T: Serialize + Send + Sync>(&self, key: &str, value: &T) -> Result<(), StorageError> {
        let json = serde_json::to_value(value)
            .map_err(|e| StorageError::Serialization(e.to_string()))?;
        self.set_value(key, json).await
    }
    
    async fn get<T: DeserializeOwned>(&self, key: &str) -> Result<Option<T>, StorageError> {
        match self.get_value(key).await? {
            Some(json) => {
                let value = serde_json::from_value(json)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                Ok(Some(value))
            }
            None => Ok(None),
        }
    }
}

/// In-memory storage backend (for testing)
#[derive(Debug, Default)]
pub struct MemoryBackend {
    data: tokio::sync::RwLock<std::collections::HashMap<String, serde_json::Value>>,
}

impl MemoryBackend {
    pub fn new() -> Self {
        Self::default()
    }
}

#[async_trait]
impl StorageBackend for MemoryBackend {
    fn name(&self) -> &str {
        "memory"
    }
    
    async fn is_healthy(&self) -> bool {
        true
    }
    
    async fn set_value(&self, key: &str, value: serde_json::Value) -> Result<(), StorageError> {
        self.data.write().await.insert(key.to_string(), value);
        Ok(())
    }
    
    async fn get_value(&self, key: &str) -> Result<Option<serde_json::Value>, StorageError> {
        let data = self.data.read().await;
        Ok(data.get(key).cloned())
    }
    
    async fn delete(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.data.write().await.remove(key).is_some())
    }
    
    async fn exists(&self, key: &str) -> Result<bool, StorageError> {
        Ok(self.data.read().await.contains_key(key))
    }
    
    async fn list_keys(&self, prefix: &str) -> Result<Vec<String>, StorageError> {
        let data = self.data.read().await;
        let keys: Vec<String> = data.keys()
            .filter(|k| k.starts_with(prefix))
            .cloned()
            .collect();
        Ok(keys)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};
    // use crate::backend::StorageExt; // Already available in super if public, but better explicit?
    // super::*; includes it if it's in parent module. Public trait methods are available if trait is in scope.


    #[derive(Debug, Serialize, Deserialize, PartialEq)]
    struct TestData {
        name: String,
        value: i32,
    }

    #[tokio::test]
    async fn test_memory_backend() {
        let backend = MemoryBackend::new();
        
        let data = TestData { name: "test".to_string(), value: 42 };
        
        // Set
        backend.set("test:1", &data).await.unwrap();
        
        // Get
        let retrieved: Option<TestData> = backend.get("test:1").await.unwrap();
        assert_eq!(retrieved, Some(data));
        
        // Exists
        assert!(backend.exists("test:1").await.unwrap());
        assert!(!backend.exists("test:2").await.unwrap());
        
        // List
        let keys = backend.list_keys("test:").await.unwrap();
        assert_eq!(keys, vec!["test:1"]);
        
        // Delete
        assert!(backend.delete("test:1").await.unwrap());
        assert!(!backend.exists("test:1").await.unwrap());
    }
}
