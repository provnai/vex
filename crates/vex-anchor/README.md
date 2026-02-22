# vex-anchor

Public anchoring layer for VEX audit logs.

## Features

- **`AnchorBackend` trait** — Pluggable anchoring backends
- **`FileAnchor`** — JSON Lines append-only log for development
- **`GitAnchor`** — Orphan branch commits for tamper-evident timestamping

## Usage

```rust
use vex_anchor::{FileAnchor, AnchorBackend, AnchorMetadata};
use vex_core::Hash;

#[tokio::main]
async fn main() {
    let anchor = FileAnchor::new("anchors.jsonl");
    
    let metadata = AnchorMetadata::new("tenant-123", 42);
    let root = Hash::digest(b"merkle root");
    
    let receipt = anchor.anchor(&root, metadata).await.unwrap();
    println!("Anchored: {}", receipt.anchor_id);
}
```

## Supported Backends

- **Ethereum** (EIP-4844 Calldata)
- **Celestia** (Blobstream/DA)
- **OpenTimestamps** (Bitcoin-backed)
- **FileAnchor** (JSON Lines for local dev)
- **GitAnchor** (Tamper-evident Git commits)

## License

MIT
