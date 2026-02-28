# vex-macros

Procedural macros for the VEX Protocol.

## Features

- **Derive Macros** - Automatic trait implementations
- **Attribute Macros** - Declarative agent configuration

## Installation

```toml
[dependencies]
vex-macros = "0.1"
```

## Quick Start

```rust
use vex_macros::Agent;

#[derive(Agent)]
struct MyAgent {
    name: String,
    state: AgentState,
}
```

## License

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
