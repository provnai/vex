//! # VEX Anchor
//!
//! Public anchoring layer for VEX audit logs.
//!
//! Provides cryptographic anchoring of Merkle roots to external systems for
//! tamper-evident, publicly-verifiable audit trails.
//!
//! ## Supported Backends (2025)
//!
//! - **FileAnchor**: Local append-only JSON log (default, for development)
//! - **GitAnchor**: Commits roots to a Git repository
//! - *Future*: EIP-4844 blobs (Ethereum), Celestia, OpenTimestamps
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use vex_anchor::{AnchorBackend, FileAnchor, AnchorMetadata};
//! use vex_core::Hash;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let anchor = FileAnchor::new("./anchors.json");
//!     
//!     let root = Hash::digest(b"merkle_root_data");
//!     let metadata = AnchorMetadata::new("tenant-1", 100);
//!     
//!     let receipt = anchor.anchor(&root, metadata).await?;
//!     println!("Anchored at: {}", receipt.anchor_id);
//!     
//!     Ok(())
//! }
//! ```

mod backend;
mod error;

#[cfg(feature = "file")]
mod file;

#[cfg(feature = "git")]
mod git;

pub use backend::{AnchorBackend, AnchorMetadata, AnchorReceipt};
pub use error::AnchorError;

#[cfg(feature = "file")]
pub use file::FileAnchor;

#[cfg(feature = "git")]
pub use git::GitAnchor;
