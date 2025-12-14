# VEX Protocol Documentation

> Verified Evolutionary Xenogenesis — A Rust framework for adversarial, temporal, cryptographically-verified AI agents.

## Quick Links

- [Getting Started](getting-started.md)
- [Architecture Overview](architecture.md)
- [API Reference](https://provnai.dev/vex_core/)
- [GitHub Repository](https://github.com/provnai/vex)

## What is VEX?

VEX is an open-source verification and memory layer for LLM agents. It solves three critical problems:

| Problem | Solution |
|---------|----------|
| **Hallucination** | Adversarial Red/Blue agent verification |
| **Context Overflow** | Bio-inspired temporal memory with decay |
| **Unauditability** | Merkle tree hash chains with tamper-evident proofs |

## Crates

| Crate | Purpose |
|-------|---------|
| `vex-core` | Agent, Genome, Merkle, ContextPacket |
| `vex-adversarial` | Red/Blue debate, consensus protocols |
| `vex-temporal` | Episodic memory, time horizons |
| `vex-llm` | LLM providers (DeepSeek, OpenAI, Ollama) |
| `vex-api` | HTTP API with JWT auth |
| `vex-persist` | SQLite storage |
| `vex-queue` | Background job processing |
| `vex-runtime` | Agent orchestration |

## Getting Help

- [GitHub Issues](https://github.com/provnai/vex/issues)
- [Contributing Guide](https://github.com/provnai/vex/blob/main/CONTRIBUTING.md)
