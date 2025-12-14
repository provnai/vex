# VEX Benchmarks

> **Verification Date**: 2025-12-14
> **System**: VEX Scale Test Suite Local Microbenchmarks (Ryzen 9 3900X)

---

## 1. Micro-Benchmarks (Component Level)

These benchmarks measure the raw efficiency of core internal structures using `criterion`.

### Merkle Tree Operations
Core cryptographic structure for agent identity and audit logs.

| Operation | Input Size | Time / Op | Throughput | 
|-----------|------------|-----------|------------|
| **Creation** | 1,000 leaves | 191 µs | ~5,200 trees/sec |
| **Creation** | 10,000 leaves | 1.97 ms | ~500 trees/sec |
| **Verification** | 10,000 leaves | 1.63 µs | **613,000 ops/sec** |

> **Implication**: The cryptographic layer (even with 10k history items) introduces negligible latency (< 2ms).

### Agent Instantiation
Cost of spawning a new agent structure (excluding LLM I/O).

| Operation | Time | Notes |
|-----------|------|-------|
| `Agent::new()` | 450 ns | Zero-copy genome creation |
| `Agent::spawn_child()` | 600 ns | Includes genome crossover/mutation logic |

---

## 2. Macro-Benchmarks (System Level)

These "Scale & Integration Tests" measured the full system performance including network latency (DeepSeek API), database persistence (SQLite), and asymptotic memory decay.

**Environment**:
- **Concurrency**: `tokio` multi-threaded runtime
- **Network**: Live API calls (DeepSeek v3)
- **State**: Full persistence enabled (WAL mode)

| Metric | Result | Analysis |
|--------|--------|----------|
| **Baseline Latency (1 Agent)** | ~1.6s | Pure API Network RTT + Minimal Overhead. |
| **Concurrent Latency (5 Agents)** | **~3.0s** | Only ~1.4s overhead for 5x work. Proves non-blocking I/O. |
| **Heavy Load Latency (10 Agents)** | ~21.9s | Includes 3-round debates per agent. 100% Reliability. |
| **Throughput (Code Gen)** | **~4.8 KB/sec** | Generated 39KB of Rust code across 10 agents in ~8s. |

---

## 3. Methodology

### Micro-Benchmarks
Run via `cargo bench` on a Linux/WSL workstation.
```bash
cargo bench -p vex-core
```

### Macro-Benchmarks
Run via `vex-scale-tests` binary on a cloud instance.
```bash
# Level 7 Verification Run
export DEEPSEEK_API_KEY="..."
cargo run --release --bin vex-scale-tests -- all
```
