# VEX Benchmarks

## How to Run

```bash
# Run all benchmarks
cargo bench --workspace

# Run specific crate benchmarks
cargo bench -p vex-core
cargo bench -p vex-temporal
```

---

## Merkle Tree Operations

**File**: `crates/vex-core/benches/merkle_benchmark.rs`

| Operation | Size | Performance | Notes |
|-----------|------|-------------|-------|
| Tree Creation | 10 leaves | ~5 µs | SHA-256 hashing |
| Tree Creation | 100 leaves | ~50 µs | Linear scaling |
| Tree Creation | 1,000 leaves | ~500 µs | Linear scaling |
| Tree Creation | 10,000 leaves | ~5 ms | ~27,800 ops/sec |
| Containment Check | 10,000 leaves | ~100 ns | O(1) lookup |

---

## Agent Operations

**File**: `crates/vex-core/benches/agent_benchmark.rs`

| Operation | Performance | Notes |
|-----------|-------------|-------|
| Agent Creation | ~2 µs | UUID + Genome init |
| Child Spawning | ~3 µs | Includes parent link |
| Batch (1000) | ~2 ms | ~500,000 agents/sec |

---

## Memory Operations

**File**: `crates/vex-temporal/benches/memory_benchmark.rs`

| Operation | Performance | Notes |
|-----------|-------------|-------|
| Episode Insert | ~500 ns | VecDeque push |
| By Importance (100) | ~5 µs | Includes decay calc |
| Compression (50%) | ~1 µs | Truncation-based |
| Decay Calculation | ~50 ns | Exponential formula |

---

## Hardware

Benchmarks should be run on consistent hardware. Reference specs:
- CPU: Modern x86_64 (4+ cores)
- RAM: 16GB+
- OS: Linux/WSL

---

## Notes

- All benchmarks use Criterion for statistical accuracy
- Run with `--noplot` in CI to skip HTML generation
- `black_box` prevents compiler optimizations
