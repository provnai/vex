# vex-api

Industry-grade HTTP API gateway for the VEX Protocol.

## Features

- **RESTful API** - Full CRUD operations for agents
- **JWT Authentication** - Secure API access
- **Rate Limiting** - Protect against abuse
- **Circuit Breaker** - Resilient external service calls
- **OpenTelemetry** - Production observability

## Installation

```toml
[dependencies]
vex-api = "0.1"
```

## Quick Start

```rust
use vex_api::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = Server::new()
        .bind("0.0.0.0:3000")
        .build()
        .await?;
    
    server.run().await?;
    Ok(())
}
```

## API Endpoints

- `POST /agents` - Create a new agent
- `GET /agents/:id` - Get agent by ID
- `POST /agents/:id/run` - Execute agent task
- `GET /health` - Health check

## License

MIT License - see [LICENSE](../../LICENSE) for details.
