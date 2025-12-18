//! Git-based anchoring backend
//!
//! Commits Merkle roots to a Git repository for tamper-evident timestamping.
//! Each anchor creates a new commit with the root hash in the commit message.

use async_trait::async_trait;
use chrono::Utc;
use std::path::PathBuf;
use std::process::Stdio;
use tokio::process::Command;
use vex_core::Hash;

use crate::backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
use crate::error::AnchorError;

/// Git-based anchor backend
///
/// Creates commits in a Git repository containing the Merkle root.
/// The commit hash serves as a tamper-evident timestamp.
///
/// ## Security Properties
/// - Git commit hashes are SHA-1 (legacy) or SHA-256 (modern)
/// - Commits can be pushed to remote repositories for redundancy
/// - OpenTimestamps can be added for Bitcoin-backed timestamps
#[derive(Debug, Clone)]
pub struct GitAnchor {
    repo_path: PathBuf,
    branch: String,
}

impl GitAnchor {
    /// Create a new Git anchor
    pub fn new(repo_path: impl Into<PathBuf>) -> Self {
        Self {
            repo_path: repo_path.into(),
            branch: "vex-anchors".to_string(),
        }
    }

    /// Set the branch to use for anchors
    pub fn with_branch(mut self, branch: impl Into<String>) -> Self {
        self.branch = branch.into();
        self
    }

    /// Sanitize a string for use in git commit messages
    /// Prevents log injection and potential git hook exploitation (CRITICAL-2 fix)
    fn sanitize_git_message(s: &str) -> String {
        s.chars()
            // Allow alphanumeric, common punctuation, and whitespace
            .filter(|c| c.is_alphanumeric() || " -_:.@/()[]{}".contains(*c))
            // Remove control characters and ANSI escape sequences
            .filter(|c| !c.is_control())
            // Limit length to prevent abuse
            .take(1000)
            .collect()
    }

    /// Run a git command and return stdout
    async fn git(&self, args: &[&str]) -> Result<String, AnchorError> {
        let output = Command::new("git")
            .args(args)
            .current_dir(&self.repo_path)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AnchorError::Git(format!("Failed to run git: {}", e)))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(AnchorError::Git(format!("Git command failed: {}", stderr)));
        }

        Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
    }

    /// Initialize the anchor branch if it doesn't exist
    async fn ensure_branch(&self) -> Result<(), AnchorError> {
        // Check if branch exists
        let branches = self.git(&["branch", "--list", &self.branch]).await?;
        
        if branches.is_empty() {
            // Create orphan branch for anchors
            self.git(&["checkout", "--orphan", &self.branch]).await?;
            
            // Create initial commit
            self.git(&["commit", "--allow-empty", "-m", "VEX Anchor Chain Initialized"]).await?;
        } else {
            // Switch to the branch
            self.git(&["checkout", &self.branch]).await?;
        }

        Ok(())
    }
}

#[async_trait]
impl AnchorBackend for GitAnchor {
    async fn anchor(
        &self,
        root: &Hash,
        metadata: AnchorMetadata,
    ) -> Result<AnchorReceipt, AnchorError> {
        // Ensure we're on the right branch
        self.ensure_branch().await?;

        // Sanitize user-controlled fields (CRITICAL-2 fix)
        let safe_tenant = Self::sanitize_git_message(&metadata.tenant_id);
        let safe_description = Self::sanitize_git_message(
            metadata.description.as_deref().unwrap_or("N/A")
        );

        // Create commit message with structured data
        let message = format!(
            "VEX Anchor: {}\n\n\
            Root: {}\n\
            Tenant: {}\n\
            Events: {}\n\
            Timestamp: {}\n\
            Description: {}",
            &root.to_hex()[..16],
            root.to_hex(),
            safe_tenant,
            metadata.event_count,
            metadata.timestamp.to_rfc3339(),
            safe_description
        );

        // Create empty commit with the anchor data
        let commit_hash = self.git(&[
            "commit",
            "--allow-empty",
            "-m",
            &message,
        ]).await?;

        // Get the commit hash
        let anchor_id = self.git(&["rev-parse", "HEAD"]).await?;

        Ok(AnchorReceipt {
            backend: self.name().to_string(),
            root_hash: root.to_hex(),
            anchor_id,
            anchored_at: Utc::now(),
            proof: Some(format!("git:{}:{}", self.branch, commit_hash)),
            metadata,
        })
    }

    async fn verify(&self, receipt: &AnchorReceipt) -> Result<bool, AnchorError> {
        // Switch to anchor branch
        let _ = self.git(&["checkout", &self.branch]).await;

        // Check if commit exists
        let result = self.git(&["cat-file", "-t", &receipt.anchor_id]).await;
        
        if result.is_err() {
            return Ok(false);
        }

        // Get commit message
        let message = self.git(&["log", "-1", "--format=%B", &receipt.anchor_id]).await?;

        // Verify root hash is in the commit
        Ok(message.contains(&receipt.root_hash))
    }

    fn name(&self) -> &str {
        "git"
    }

    async fn is_healthy(&self) -> bool {
        // Check if this is a git repository
        self.git(&["status"]).await.is_ok()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    async fn init_test_repo(path: &PathBuf) {
        Command::new("git")
            .args(["init"])
            .current_dir(path)
            .output()
            .await
            .unwrap();
        
        Command::new("git")
            .args(["config", "user.email", "test@test.com"])
            .current_dir(path)
            .output()
            .await
            .unwrap();
        
        Command::new("git")
            .args(["config", "user.name", "Test"])
            .current_dir(path)
            .output()
            .await
            .unwrap();

        // Initial commit on main
        Command::new("git")
            .args(["commit", "--allow-empty", "-m", "Initial"])
            .current_dir(path)
            .output()
            .await
            .unwrap();
    }

    #[tokio::test]
    async fn test_git_anchor_roundtrip() {
        let dir = tempdir().unwrap();
        let repo_path = dir.path().to_path_buf();
        init_test_repo(&repo_path).await;

        let anchor = GitAnchor::new(&repo_path);

        let root = Hash::digest(b"test_merkle_root");
        let metadata = AnchorMetadata::new("tenant-1", 100);

        // Anchor
        let receipt = anchor.anchor(&root, metadata).await.unwrap();
        assert_eq!(receipt.backend, "git");
        assert_eq!(receipt.root_hash, root.to_hex());
        assert!(!receipt.anchor_id.is_empty());

        // Verify
        let valid = anchor.verify(&receipt).await.unwrap();
        assert!(valid, "Receipt should verify");
    }
}
