# vex-algoswitch

Runtime algorithm selection and optimization for high-performance VEX operations.

`vex-algoswitch` provides a framework for dynamically selecting the most efficient algorithmic path based on data traits (size, entropy, type) and history.

## Features

- **Smart Selection** - Choose between recursive, iterative, or parallel strategies at runtime.
- **Pattern Recognition** - Analyzes data patterns to predict the optimal algorithm (e.g., small arrays vs. large arrays).
- **Zero-Overhead Thresholds** - Minimal runtime cost for algorithm switching.
- **VEX Integration** - Optimized for Merkle tree traversals and non-critical audit hashing.

## Installation

```toml
[dependencies]
vex-algoswitch = "0.1"
```

## Quick Start

```rust
use vex_algoswitch::{select, Config};

fn main() {
    let mut data = vec![5, 3, 8, 1, 9];
    let algos = vec![
        ("quicksort", vex_algoswitch::sort::quicksort as fn(&mut [i64])),
        ("mergesort", vex_algoswitch::sort::mergesort as fn(&mut [i64])),
    ];
    
    let result = select(algos, &mut data, Config::default());
    println!("Selected: {}", result.algorithm_name);
}
```

## License

Apache-2.0 License - see [LICENSE](../../LICENSE) for details.
