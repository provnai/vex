# Getting Started with VEX

This guide will help you get up and running with VEX in under 5 minutes.

## Prerequisites

- **Rust 1.75+** (stable toolchain)
- **Go 1.22+** (for hardware-rooted identity via `attest`)
- **Git**
- **TPM 2.0 / Microsoft CNG** (Required for secure identity)

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
# Supports: DEEPSEEK_API_KEY, MISTRAL_API_KEY, or OPENAI_API_KEY
export DEEPSEEK_API_KEY="sk-..."
# or
export MISTRAL_API_KEY="your-mistral-key"

# Run the CLI tool to list available tools
cargo run -p vex-protocol-cli -- tools list

# Run a tool via CLI
cargo run -p vex-protocol-cli -- tools run calculator '{"expression": "2+2"}'
```

## Running the API Server

```bash
export VEX_JWT_SECRET="your-32-char-secret-here"
cargo run -p vex-api
# Server starts on 0.0.0.0:8080
```

### Real-time Status (v0.2.0)

Follow job progress via Server-Sent Events:

```bash
curl -N -H "Authorization: Bearer <token>" \
     http://localhost:3000/api/v1/jobs/<job_id>/stream
```

## Next Steps

- [Architecture Overview](../ARCHITECTURE.md)
- [API Reference](https://www.provnai.dev/docs)
- [Contributing](../CONTRIBUTING.md)
