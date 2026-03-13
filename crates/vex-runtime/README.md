# vex-runtime

Tokio-based agent orchestration for the VEX Protocol.

## Features

- **Agent Orchestration** - Run multiple agents concurrently
- **Formal Intent Verification** - L2 Semantic verification via Magpie AST
- **WSL Interop** - Full cross-platform support for Windows/Linux/WSL
- **Task Scheduling** - Prioritized agent task execution

## Installation

```toml
[dependencies]
[dependencies]
vex-runtime = "1.3.0"
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

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
