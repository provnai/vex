//! File-based anchoring backend
//!
//! Appends anchor receipts to a local JSON file for development and testing.

use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;
use vex_core::Hash;

use crate::backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
use crate::error::AnchorError;

/// File-based anchor backend
///
/// Stores anchor receipts in a local JSON Lines file (one receipt per line).
/// Suitable for development, testing, and single-node deployments.
///
/// # Security
/// Use `try_new()` or `with_base_dir()` for production to prevent path traversal.
#[derive(Debug, Clone)]
pub struct FileAnchor {
    path: PathBuf,
}

impl FileAnchor {
    /// Create a new file anchor (no path validation)
    ///
    /// # Warning
    /// This constructor does not validate the path. For production use,
    /// prefer `try_new()` or `with_base_dir()` to prevent path traversal.
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self { path: path.into() }
    }

    /// Create a file anchor with path validation (HIGH-1 fix)
    ///
    /// Validates that the path:
    /// - Does not contain path traversal sequences (`..`)
    /// - Is within the specified base directory
    ///
    /// # Errors
    /// Returns `AnchorError::BackendUnavailable` if path validation fails.
    pub fn with_base_dir(
        path: impl Into<PathBuf>,
        base_dir: impl Into<PathBuf>,
    ) -> Result<Self, AnchorError> {
        let path = path.into();
        let base_dir = base_dir.into();

        // Check for path traversal in the filename/path components
        let path_str = path.to_string_lossy();
        if path_str.contains("..") {
            return Err(AnchorError::BackendUnavailable(
                "Path traversal detected: '..' not allowed in anchor path".to_string(),
            ));
        }

        // Ensure the path is under the base directory
        // Use the raw path if canonicalize fails (file may not exist yet)
        let resolved = if path.is_absolute() {
            path.clone()
        } else {
            base_dir.join(&path)
        };

        // Check that resolved path starts with base_dir
        // This prevents ../../../etc/passwd style attacks
        let base_canonical = base_dir.canonicalize().unwrap_or(base_dir);
        let resolved_parent = resolved
            .parent()
            .map(|p| p.to_path_buf())
            .unwrap_or(resolved.clone());

        if !resolved_parent.starts_with(&base_canonical) && resolved_parent != base_canonical {
            // For new files, check if the parent would be valid
            let parent_str = resolved_parent.to_string_lossy();
            if !parent_str.starts_with(&base_canonical.to_string_lossy().as_ref()) {
                return Err(AnchorError::BackendUnavailable(format!(
                    "Path '{}' is outside allowed directory '{}'",
                    resolved.display(),
                    base_canonical.display()
                )));
            }
        }

        Ok(Self { path: resolved })
    }

    /// Get the path to the anchor file
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

#[async_trait]
impl AnchorBackend for FileAnchor {
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        let anchor_id = uuid::Uuid::new_v4().to_string();
        let receipt = AnchorReceipt {
            backend: self.name().to_string(),
            root_hash: root.to_hex(),
            anchor_id: anchor_id.clone(),
            anchored_at: Utc::now(),
            proof: None,
            metadata,
        };

        // Append to file (JSON Lines format)
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.path)
            .await?;

        let mut json = serde_json::to_string(&receipt)?;
        json.push('\n');
        file.write_all(json.as_bytes()).await?;
        file.flush().await?;

        Ok(receipt)
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        use subtle::ConstantTimeEq;

        if !self.path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&self.path).await?;

        for line in content.lines() {
            if line.trim().is_empty() {
                continue;
            }

            let parsed: Result<AnchorReceipt, _> = serde_json::from_str(line);
            if let Ok(stored) = parsed {
                // Use constant-time comparison to prevent timing attacks (LOW-1 fix)
                let id_match = stored
                    .anchor_id
                    .as_bytes()
                    .ct_eq(receipt.anchor_id.as_bytes());
                let hash_match = stored
                    .root_hash
                    .as_bytes()
                    .ct_eq(receipt.root_hash.as_bytes());

                if id_match.into() && hash_match.into() {
                    return Ok(true);
                }
            }
        }

        Ok(false)
    }

    fn name(&self) -> &str {
        "file"
    }

    async fn is_healthy(&self) -> bool {
        // Check if we can write to the directory
        if let Some(parent) = self.path.parent() {
            if !parent.exists() {
                return fs::create_dir_all(parent).await.is_ok();
            }
        }
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[tokio::test]
    async fn test_file_anchor_roundtrip() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("anchors.jsonl");
        let anchor = FileAnchor::new(&path);

        let root = Hash::digest(b"test_merkle_root");
        let metadata = AnchorMetadata::new("tenant-1", 100);

        // Anchor
        let receipt = anchor.anchor(&root, metadata).await.unwrap();
        assert_eq!(receipt.backend, "file");
        assert_eq!(receipt.root_hash, root.to_hex());

        // Verify
        let valid = anchor.verify(&receipt).await.unwrap();
        assert!(valid, "Receipt should verify");

        // Invalid receipt should not verify
        let mut fake = receipt.clone();
        fake.anchor_id = "fake-id".to_string();
        let invalid = anchor.verify(&fake).await.unwrap();
        assert!(!invalid, "Fake receipt should not verify");
    }

    #[tokio::test]
    async fn test_file_anchor_multiple() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("anchors.jsonl");
        let anchor = FileAnchor::new(&path);

        let mut receipts = Vec::new();
        for i in 0..5 {
            let root = Hash::digest(format!("root_{}", i).as_bytes());
            let metadata = AnchorMetadata::new("tenant-1", i as u64);
            let receipt = anchor.anchor(&root, metadata).await.unwrap();
            receipts.push(receipt);
        }

        // All should verify
        for receipt in &receipts {
            assert!(anchor.verify(receipt).await.unwrap());
        }

        // Check file has 5 lines
        let content = fs::read_to_string(&path).await.unwrap();
        assert_eq!(content.lines().count(), 5);
    }
}
