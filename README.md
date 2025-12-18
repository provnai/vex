# VEX Protocol

> **The trust layer for AI agents.**

Adversarial verification • Temporal memory • Cryptographic proofs — all in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/vex-core.svg)](https://crates.io/crates/vex-core)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/provnai/vex/workflows/CI/badge.svg)](https://github.com/provnai/vex/actions)
[![Docs](https://img.shields.io/badge/docs-provnai.dev-4285F4.svg)](https://www.provnai.dev/docs)
[![Website](https://img.shields.io/badge/website-provnai.com-00C7B7.svg)](https://provnai.com)

---

## Why VEX?

| Problem | VEX Solution |
|---------|--------------|
| **Hallucination** | Red/Blue adversarial debate with consensus |
| **Context Overflow** | Bio-inspired temporal memory with decay |
| **Unauditability** | Merkle hash chains with tamper-evident proofs |

VEX is a verification and memory layer that works with any LLM provider.

📚 **[Full Documentation →](https://www.provnai.dev/docs)**

---

## Quick Start

```bash
# Build
cargo build --workspace --release

# Test (85+ tests)
cargo test --workspace

# Run demo
cargo run -p vex-demo

# CLI
cargo run -p vex-cli -- tools list
cargo run -p vex-cli -- tools run calculator '{"expression": "2+2"}'
```

### Environment Variables

```bash
export DEEPSEEK_API_KEY="sk-..."     # Or MISTRAL_API_KEY, OPENAI_API_KEY
export VEX_JWT_SECRET="your-32-char-secret"
```

---

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│  vex-api     │ HTTP Gateway, JWT, Rate Limiting        │
├──────────────┼──────────────────────────────────────────┤
│  vex-llm     │ DeepSeek, Mistral, OpenAI, Ollama, Tools│
│  vex-adv     │ Red/Blue Debate, Consensus Protocols    │
├──────────────┼──────────────────────────────────────────┤
│  vex-runtime │ Orchestrator, Self-Correcting Genome    │
│  vex-queue   │ Async Worker Pool                       │
├──────────────┼──────────────────────────────────────────┤
│  vex-core    │ Agent, Genome, Merkle Tree, Evolution   │
│  vex-temporal│ Episodic Memory, Decay Strategies       │
├──────────────┼──────────────────────────────────────────┤
│  vex-persist │ SQLite, Audit Logs, Hash Chains         │
└──────────────┴──────────────────────────────────────────┘
```

📐 **[Full Architecture →](https://www.provnai.dev/docs/architecture)**

---

## Key Features

### Adversarial Verification
Blue Agent → Red Agent Challenge → Rebuttal → Consensus

### Temporal Memory
5 horizons (Immediate → Permanent) with configurable decay

### Cryptographic Audit
SHA-256 hash chains, Merkle proofs, tamper detection

### Tool System
6 built-in tools + MCP client + A2A protocol support

### Self-Correcting Genome
Autonomous trait optimization with persistent learning

📖 **[All Features →](https://www.provnai.dev/docs)**

---

## Workspace

| Crate | Purpose |
|-------|---------|
| `vex-core` | Agent, Genome, Merkle, Evolution |
| `vex-adversarial` | Debate, Consensus, Reflection |
| `vex-temporal` | Memory, Decay, Compression |
| `vex-llm` | LLM Providers, Tools, MCP |
| `vex-api` | HTTP Server, Auth, A2A |
| `vex-runtime` | Orchestrator, Self-Correction |
| `vex-persist` | SQLite, Audit Store |
| `vex-cli` | Command-line Interface |

---

## Documentation

| Resource | Link |
|----------|------|
| **Full Docs** | [provnai.dev/docs](https://www.provnai.dev/docs) |
| **API Reference** | [provnai.dev/rustdoc](https://www.provnai.dev/rustdoc) |
| **Architecture** | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Benchmarks** | [BENCHMARKS.md](BENCHMARKS.md) |
| **Contributing** | [CONTRIBUTING.md](CONTRIBUTING.md) |

---

## License

MIT — See [LICENSE](LICENSE)
