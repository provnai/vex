# vex-adversarial

Adversarial (Red/Blue) agent pairing for the VEX Protocol.

## Features

- **Shadow Agents** - Monitor and challenge primary agent decisions
- **Debate System** - Structured adversarial dialogues between agents
- **Consensus** - Multi-agent agreement protocols

## Installation

```toml
[dependencies]
vex-adversarial = "0.1"
```

## Quick Start

```rust
use vex_adversarial::{ShadowAgent, Debate};
use vex_core::Agent;

#[tokio::main]
async fn main() {
    let primary = Agent::new("primary");
    let shadow = ShadowAgent::new("shadow", &primary);
    
    // Shadow agent monitors and challenges primary decisions
}
```

## License

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
