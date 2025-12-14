//! Merkle tree implementation for context verification
//!
//! Provides cryptographic verification of context packet hierarchies.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::fmt;

/// A SHA-256 hash (32 bytes)
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Hash(pub [u8; 32]);

impl Hash {
    /// Create a hash from raw bytes
    pub fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    /// Hash arbitrary data
    pub fn digest(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    /// Combine two hashes (for Merkle tree internal nodes)
    pub fn combine(left: &Hash, right: &Hash) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(left.0);
        hasher.update(right.0);
        Self(hasher.finalize().into())
    }

    /// Get hex representation
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }
}

impl fmt::Debug for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Hash({})", &self.to_hex()[..16])
    }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", &self.to_hex()[..16])
    }
}

/// A node in the Merkle tree
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MerkleNode {
    /// Leaf node containing actual data hash
    Leaf { hash: Hash, data_id: String },
    /// Internal node combining two child hashes
    Internal {
        hash: Hash,
        left: Box<MerkleNode>,
        right: Box<MerkleNode>,
    },
}

impl MerkleNode {
    /// Get the hash of this node
    pub fn hash(&self) -> &Hash {
        match self {
            Self::Leaf { hash, .. } => hash,
            Self::Internal { hash, .. } => hash,
        }
    }
}

/// A Merkle tree for verifying context packet integrity
#[derive(Debug, Clone)]
pub struct MerkleTree {
    root: Option<MerkleNode>,
    leaf_count: usize,
}

impl MerkleTree {
    /// Create an empty Merkle tree
    pub fn new() -> Self {
        Self {
            root: None,
            leaf_count: 0,
        }
    }

    /// Build a Merkle tree from a list of (id, hash) pairs (zero-copy construction)
    pub fn from_leaves(leaves: Vec<(String, Hash)>) -> Self {
        if leaves.is_empty() {
            return Self::new();
        }

        let leaf_count = leaves.len();
        let mut nodes: Vec<MerkleNode> = leaves
            .into_iter()
            .map(|(data_id, hash)| MerkleNode::Leaf { hash, data_id })
            .collect();

        // Build tree bottom-up using move semantics (no cloning)
        while nodes.len() > 1 {
            let mut next_level = Vec::with_capacity(nodes.len().div_ceil(2));
            let mut iter = nodes.into_iter();

            while let Some(left_node) = iter.next() {
                if let Some(right_node) = iter.next() {
                    let combined_hash = Hash::combine(left_node.hash(), right_node.hash());
                    next_level.push(MerkleNode::Internal {
                        hash: combined_hash,
                        left: Box::new(left_node),
                        right: Box::new(right_node),
                    });
                } else {
                    // Odd number of nodes, carry the last one up
                    next_level.push(left_node);
                }
            }

            nodes = next_level;
        }

        Self {
            root: nodes.into_iter().next(),
            leaf_count,
        }
    }

    /// Get the root hash (None if tree is empty)
    pub fn root_hash(&self) -> Option<&Hash> {
        self.root.as_ref().map(|n| n.hash())
    }

    /// Get the number of leaves
    pub fn len(&self) -> usize {
        self.leaf_count
    }

    /// Check if tree is empty
    pub fn is_empty(&self) -> bool {
        self.leaf_count == 0
    }

    /// Verify that a hash is part of this tree (zero-copy traversal)
    pub fn contains(&self, target_hash: &Hash) -> bool {
        match &self.root {
            None => false,
            Some(node) => Self::contains_node(node, target_hash),
        }
    }

    /// Recursive helper that takes a reference - no cloning needed
    fn contains_node(node: &MerkleNode, target: &Hash) -> bool {
        match node {
            MerkleNode::Leaf { hash, .. } => hash == target,
            MerkleNode::Internal { hash, left, right } => {
                hash == target
                    || Self::contains_node(left, target)
                    || Self::contains_node(right, target)
            }
        }
    }
}

impl Default for MerkleTree {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_combine() {
        let h1 = Hash::digest(b"hello");
        let h2 = Hash::digest(b"world");
        let combined = Hash::combine(&h1, &h2);

        // Combining same hashes should give same result
        let combined2 = Hash::combine(&h1, &h2);
        assert_eq!(combined, combined2);
    }

    #[test]
    fn test_merkle_tree() {
        let leaves = vec![
            ("a".to_string(), Hash::digest(b"data_a")),
            ("b".to_string(), Hash::digest(b"data_b")),
            ("c".to_string(), Hash::digest(b"data_c")),
            ("d".to_string(), Hash::digest(b"data_d")),
        ];

        let tree = MerkleTree::from_leaves(leaves.clone());
        assert_eq!(tree.len(), 4);
        assert!(tree.root_hash().is_some());

        // Should find all original hashes
        for (_, hash) in &leaves {
            assert!(tree.contains(hash));
        }
    }
}
