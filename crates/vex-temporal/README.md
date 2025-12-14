# vex-temporal

Time-aware memory compression for the VEX Protocol.

## Features

- **Temporal Compression** - Intelligent context summarization over time
- **Memory Decay** - Configurable importance-weighted memory retention
- **Context Windows** - Efficient LLM context management

## Installation

```toml
[dependencies]
vex-temporal = "0.1"
```

## Quick Start

```rust
use vex_temporal::TemporalMemory;
use vex_core::ContextPacket;

#[tokio::main]
async fn main() {
    let mut memory = TemporalMemory::new();
    
    // Add contexts - older ones get compressed automatically
    memory.add_context(ContextPacket::new("Recent event")).await;
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
