# Getting Started with VEX

This guide will help you get up and running with VEX in under 5 minutes.

## Prerequisites

- **Rust 1.75+** (stable toolchain)
- **Git**

## Installation

### Add to Your Project

```toml
# Cargo.toml
[dependencies]
vex-core = { git = "https://github.com/provnai/vex" }
vex-adversarial = { git = "https://github.com/provnai/vex" }
vex-llm = { git = "https://github.com/provnai/vex" }
```

### Clone and Build

```bash
git clone https://github.com/provnai/vex.git
cd vex
cargo build --workspace --release
```

## Quick Example

```rust
use vex_core::{Agent, AgentConfig};
use vex_llm::MockProvider;

#[tokio::main]
async fn main() {
    // Create an agent
    let agent = Agent::new(AgentConfig {
        name: "Researcher".to_string(),
        role: "You are a helpful research assistant".to_string(),
        max_depth: 3,
        spawn_shadow: true,
    });

    // Use with an LLM provider
    let llm = MockProvider::smart();
    let response = llm.ask("What is quantum computing?").await.unwrap();
    
    println!("Response: {}", response);
}
```

## Running the Demo

```bash
# Set up API key (optional, uses mock if not set)
export DEEPSEEK_API_KEY="sk-..."

# Run the research agent demo
cargo run -p vex-demo

# Run fraud detection demo
cargo run -p vex-demo --bin fraud-detector

# Interactive chat
cargo run -p vex-demo --bin interactive
```

## Running the API Server

```bash
export VEX_JWT_SECRET="your-32-char-secret-here"
cargo run -p vex-api
# Server starts on 0.0.0.0:3000
```

## Next Steps

- [Architecture Overview](architecture.md)
- [API Reference](https://provnai.dev/vex_core/)
- [Contributing](https://github.com/provnai/vex/blob/main/CONTRIBUTING.md)
