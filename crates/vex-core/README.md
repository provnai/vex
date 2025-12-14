# vex-core

Core types and primitives for the VEX (Verified Evolutionary Xenogenesis) Protocol.

## Features

- **Agent** - Autonomous AI agent with cryptographic identity
- **ContextPacket** - Immutable, versioned context for agent memory
- **MerkleNode** - Cryptographic verification of agent state history
- **Evolution** - Trait-based agent evolution and improvement tracking

## Installation

```toml
[dependencies]
vex-core = "0.1"
```

## Quick Start

```rust
use vex_core::{Agent, ContextPacket};

#[tokio::main]
async fn main() {
    let agent = Agent::new("my-agent");
    let context = ContextPacket::new("Initial context");
    // ... use agent
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
