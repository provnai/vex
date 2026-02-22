//! Provides cryptographic verification of context packet hierarchies.

#[cfg(feature = "algoswitch")]
use vex_algoswitch as algoswitch;

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

    /// Hash arbitrary data (Leaf node domain separation: 0x00)
    pub fn digest(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update([0x00]); // Leaf prefix
        hasher.update(data);
        Self(hasher.finalize().into())
    }

    /// Combine two hashes (Internal node domain separation: 0x01)
    pub fn combine(left: &Hash, right: &Hash) -> Self {
        let mut hasher = Sha256::new();
        hasher.update([0x01]); // Internal prefix
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

/// Direction indicator for Merkle proof steps
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ProofDirection {
    /// Sibling hash is on the left
    Left,
    /// Sibling hash is on the right
    Right,
}

/// A single step in a Merkle inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProofStep {
    /// The sibling hash at this level
    pub sibling_hash: Hash,
    /// Whether the sibling is on the left or right
    pub direction: ProofDirection,
}

/// A Merkle inclusion proof (RFC 6962 compatible)
/// Allows proving that a leaf is part of a tree without revealing other leaves
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleProof {
    /// The leaf hash being proven
    pub leaf_hash: Hash,
    /// The leaf's data ID
    pub leaf_id: String,
    /// The path from leaf to root (bottom to top)
    pub path: Vec<ProofStep>,
    /// The expected root hash
    pub root_hash: Hash,
}

impl MerkleProof {
    /// Verify this proof against a root hash
    pub fn verify(&self, expected_root: &Hash) -> bool {
        if &self.root_hash != expected_root {
            return false;
        }

        let mut current_hash = self.leaf_hash.clone();

        for step in &self.path {
            current_hash = match step.direction {
                ProofDirection::Left => Hash::combine(&step.sibling_hash, &current_hash),
                ProofDirection::Right => Hash::combine(&current_hash, &step.sibling_hash),
            };
        }

        &current_hash == expected_root
    }

    /// Export proof as a compact JSON string for transmission
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string(self)
    }

    /// Default maximum size for proof JSON (1 MB)
    /// This limits the number of proof steps to prevent DoS
    pub const MAX_PROOF_JSON_SIZE: usize = 1024 * 1024;

    /// Import proof from JSON string with size limit (MEDIUM-2 fix)
    ///
    /// # Arguments
    /// * `json` - The JSON string to parse
    /// * `max_size` - Maximum allowed size in bytes (prevents DoS)
    ///
    /// # Errors
    /// Returns error if JSON exceeds max_size or is invalid
    pub fn from_json_with_limit(json: &str, max_size: usize) -> Result<Self, String> {
        if json.len() > max_size {
            return Err(format!(
                "Proof JSON too large: {} bytes exceeds limit of {} bytes",
                json.len(),
                max_size
            ));
        }
        serde_json::from_str(json).map_err(|e| e.to_string())
    }

    /// Import proof from JSON string with default 1MB limit
    ///
    /// For custom limits, use `from_json_with_limit()`.
    pub fn from_json(json: &str) -> Result<Self, String> {
        Self::from_json_with_limit(json, Self::MAX_PROOF_JSON_SIZE)
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

    /// Optimized search using iterative traversal (prevents stack overflow for deep trees)
    pub fn contains_iterative(&self, target_hash: &Hash) -> bool {
        let mut stack = Vec::new();
        if let Some(root) = &self.root {
            stack.push(root);
        }

        while let Some(node) = stack.pop() {
            match node {
                MerkleNode::Leaf { hash, .. } => {
                    if hash == target_hash { return true; }
                }
                MerkleNode::Internal { hash, left, right } => {
                    if hash == target_hash { return true; }
                    stack.push(right);
                    stack.push(left);
                }
            }
        }
        false
    }
    
    /// AlgoSwitch Select - picks the best search strategy based on tree size
    #[cfg(feature = "algoswitch")]
    pub fn contains_optimized(&self, target_hash: &Hash) -> bool {
        // For small trees, recursive is overhead-free (no stack allocation)
        // For large trees, iterative is safer and often faster
        if self.leaf_count < 128 {
            self.contains(target_hash)
        } else {
            self.contains_iterative(target_hash)
        }
    }

    /// Generate an inclusion proof for a leaf by its hash
    /// Returns None if the hash is not found in the tree
    pub fn get_proof_by_hash(&self, target_hash: &Hash) -> Option<MerkleProof> {
        let root = self.root.as_ref()?;
        let root_hash = root.hash().clone();

        let mut path = Vec::new();
        let (leaf_hash, leaf_id) = Self::find_path_to_hash(root, target_hash, &mut path)?;

        Some(MerkleProof {
            leaf_hash,
            leaf_id,
            path,
            root_hash,
        })
    }

    /// Helper: Find path from root to target hash, collecting sibling hashes
    fn find_path_to_hash(
        node: &MerkleNode,
        target: &Hash,
        path: &mut Vec<ProofStep>,
    ) -> Option<(Hash, String)> {
        match node {
            MerkleNode::Leaf { hash, data_id } => {
                if hash == target {
                    Some((hash.clone(), data_id.clone()))
                } else {
                    None
                }
            }
            MerkleNode::Internal { left, right, .. } => {
                // Try left subtree first
                if let Some(result) = Self::find_path_to_hash(left, target, path) {
                    // Target is in left subtree, sibling is on the right
                    path.push(ProofStep {
                        sibling_hash: right.hash().clone(),
                        direction: ProofDirection::Right,
                    });
                    return Some(result);
                }

                // Try right subtree
                if let Some(result) = Self::find_path_to_hash(right, target, path) {
                    // Target is in right subtree, sibling is on the left
                    path.push(ProofStep {
                        sibling_hash: left.hash().clone(),
                        direction: ProofDirection::Left,
                    });
                    return Some(result);
                }

                None
            }
        }
    }

    /// Verify a proof against this tree's root
    pub fn verify_proof(&self, proof: &MerkleProof) -> bool {
        match self.root_hash() {
            Some(root) => proof.verify(root),
            None => false,
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

    #[test]
    fn test_merkle_proof_generation() {
        let leaves = vec![
            ("event_1".to_string(), Hash::digest(b"data_1")),
            ("event_2".to_string(), Hash::digest(b"data_2")),
            ("event_3".to_string(), Hash::digest(b"data_3")),
            ("event_4".to_string(), Hash::digest(b"data_4")),
        ];

        let tree = MerkleTree::from_leaves(leaves.clone());
        let root = tree.root_hash().unwrap();

        // Generate and verify proof for each leaf
        for (id, hash) in &leaves {
            let proof = tree.get_proof_by_hash(hash).expect("Should find leaf");
            assert_eq!(&proof.leaf_id, id);
            assert_eq!(&proof.leaf_hash, hash);
            assert!(proof.verify(root), "Proof should verify against root");
        }
    }

    #[test]
    fn test_merkle_proof_serialization() {
        let leaves = vec![
            ("a".to_string(), Hash::digest(b"data_a")),
            ("b".to_string(), Hash::digest(b"data_b")),
        ];

        let tree = MerkleTree::from_leaves(leaves.clone());
        let proof = tree.get_proof_by_hash(&leaves[0].1).unwrap();

        // Serialize to JSON
        let json = proof.to_json().expect("Should serialize");
        assert!(json.contains("leaf_hash"));
        assert!(json.contains("path"));

        // Deserialize and verify
        let restored = MerkleProof::from_json(&json).expect("Should deserialize");
        assert_eq!(proof.leaf_id, restored.leaf_id);
        assert!(restored.verify(tree.root_hash().unwrap()));
    }

    #[test]
    fn test_merkle_proof_not_found() {
        let leaves = vec![("a".to_string(), Hash::digest(b"data_a"))];
        let tree = MerkleTree::from_leaves(leaves);

        let fake_hash = Hash::digest(b"not_in_tree");
        assert!(tree.get_proof_by_hash(&fake_hash).is_none());
    }

    #[test]
    fn test_merkle_proof_odd_leaves() {
        // Odd number of leaves - tests edge case in tree construction
        let leaves = vec![
            ("a".to_string(), Hash::digest(b"data_a")),
            ("b".to_string(), Hash::digest(b"data_b")),
            ("c".to_string(), Hash::digest(b"data_c")),
        ];

        let tree = MerkleTree::from_leaves(leaves.clone());
        let root = tree.root_hash().unwrap();

        // All proofs should still work
        for (_, hash) in &leaves {
            let proof = tree.get_proof_by_hash(hash).expect("Should find leaf");
            assert!(proof.verify(root), "Proof should verify for odd tree");
        }
    }

    #[test]
    fn test_merkle_proof_tamper_detection() {
        let leaves = vec![
            ("a".to_string(), Hash::digest(b"data_a")),
            ("b".to_string(), Hash::digest(b"data_b")),
        ];

        let tree = MerkleTree::from_leaves(leaves.clone());
        let mut proof = tree.get_proof_by_hash(&leaves[0].1).unwrap();

        // Tamper with the leaf hash
        proof.leaf_hash = Hash::digest(b"tampered");

        // Should fail verification
        assert!(
            !proof.verify(tree.root_hash().unwrap()),
            "Tampered proof should fail"
        );
    }
}
