# vex-persist

Persistence layer for the VEX Protocol.

## Features

- **SQLite Backend** - Local development and edge deployment
- **PostgreSQL Backend** - Production-ready scaling
- **Agent Store** - Persist agent state and history
- **Context Store** - Store and retrieve context packets
- **Vector Store** - SQLite-backed semantic memory (Cosine similarity)
- **Job Store** - Persistent background task results
- **Audit Trail** - Full audit logging with tamper-evident chains

## Installation

```toml
[dependencies]
# SQLite (default)
vex-persist = "0.1"

# PostgreSQL
vex-persist = { version = "0.1", features = ["postgres"] }
```

## Quick Start

```rust
use vex_persist::{SqliteBackend, AgentStore};
use vex_core::Agent;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let backend = SqliteBackend::new("vex.db").await?;
    let store = AgentStore::new(backend);
    
    let agent = Agent::new("my-agent");
    store.save(&agent).await?;
    
    Ok(())
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
