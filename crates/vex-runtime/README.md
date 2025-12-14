# vex-runtime

Tokio-based agent orchestration for the VEX Protocol.

## Features

- **Agent Orchestration** - Run multiple agents concurrently
- **Task Scheduling** - Prioritized agent task execution
- **Resource Management** - Efficient resource allocation

## Installation

```toml
[dependencies]
vex-runtime = "0.1"
```

## Quick Start

```rust
use vex_runtime::Runtime;
use vex_core::Agent;

#[tokio::main]
async fn main() {
    let runtime = Runtime::new();
    
    let agent = Agent::new("worker");
    runtime.spawn(agent).await;
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
