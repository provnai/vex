# vex-router

Intelligent LLM Routing and Observability for the VEX Protocol.

`vex-router` provides a high-performance routing layer for AI agents, enabling dynamic model selection based on cost, latency, quality, and adversarial role detection.

## Features

- **Intelligent Routing** - Route queries based on quality (GPT-4o/Sonnet) or cost (Mini/Haiku).
- **Adversarial Detection** - Automatically upgrades routing quality when red-teaming/adversarial roles are detected in the system prompt.
- **Semantic Caching** - Built-in vector-based caching to reduce redundant LLM calls and latency.
- **Observability** - Detailed metrics for tracking cost savings, latency percentiles, and request precision.
- **OpenAPI Support** - Native `utoipa` support for integration with the VEX API and Swagger UI.

## Installation

```toml
[dependencies]
vex-router = "0.1"
```

## Quick Start

```rust
use vex_router::{Router, RouterBuilder, RoutingStrategy};

#[tokio::main]
async fn main() {
    let router = RouterBuilder::new()
        .strategy(RoutingStrategy::CostOptimized)
        .build();

    let result = router.ask("Calculate the risk profile for this audit.").await;
    println!("Response: {:?}", result);
}
```

## License

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
