# vex-queue

Async background worker queue for the VEX Protocol.

## Features

- **In-Memory Backend** - Development and testing
- **Persistent Backend** - Production job durability
- **Worker Pools** - Scalable job processing
- **Job Scheduling** - Delayed and recurring jobs

## Installation

```toml
[dependencies]
vex-queue = "0.1"
```

## Quick Start

```rust
use vex_queue::{Queue, Job, Worker};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let queue = Queue::new_memory();
    
    // Enqueue a job
    queue.enqueue(Job::new("process_data", payload)).await?;
    
    // Start workers
    let worker = Worker::new(queue.clone());
    worker.run().await?;
    
    Ok(())
}
```

## License

MIT License - see [LICENSE](../../LICENSE) for details.
