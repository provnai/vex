# VEX Protocol Documentation

VEX is a Rust framework for verifying and managing AI agent state through adversarial debate, temporal memory, and cryptographic proofs.

## Quick Links

- [Getting Started](getting-started.md)
- [Deploy on Railway](railway-deployment.md)
- [Architecture Overview](../ARCHITECTURE.md)
- [API Reference](https://www.provnai.dev/docs)
- [GitHub Repository](https://github.com/provnai/vex)

## What is VEX?

| Problem | VEX Solution |
|---|---|
| **Hallucination** | Adversarial Red/Blue agent verification |
| **Outdated Traits** | Reflection cycles and agent evolution |
| **Context Overflow** | Bio-inspired temporal memory with decay |
| **Unauditability** | Merkle tree hash chains with tamper-evident Evidence Capsules |

## Technical Specifications

- [.capsule Specification v0.3](specs/capsule-v0.3.md) (Merkle Hardening)
- [.capsule Specification v0.2](specs/capsule-v0.2.md) (Standard PCR Binding - Frozen)

## Crates

| Crate | Purpose |
|-------|---------|
| `vex-core` | Agent, Genome, Merkle, ContextPacket |
| `vex-adversarial` | Red/Blue debate, Reflection agents |
| `vex-llm` | LLM providers (DeepSeek, OpenAI, Groq) |
| `vex-api` | HTTP API with JWT & SSE Streaming |
| `vex-anchor` | Merkle root anchoring |
| `vex-router` | Intelligent LLM routing & caching |
| `vex-persist` | SQLite storage & EvolutionStore |
| `vex-queue` | Background job processing |
| `vex-runtime` | Agent orchestration |
| `vex-server` | Production binary & Railway deployment |
| `vex-temporal` | Episodic memory & time horizons |
| `vex-macros` | Procedural macros for tools & audit |
| `vex-cli` | Command-line audit & authentication tools |
| `vex-sidecar` | Silicon Boundary Proxy |
| `vex-hardware` | Hardware-Rooted Identity (TPM 2.0 / Microsoft CNG) |
| `vex-algoswitch` | Self-optimizing algorithm runtime |

## Getting Help

- [GitHub Issues](https://github.com/provnai/vex/issues)
- [Contributing Guide](https://github.com/provnai/vex/blob/main/CONTRIBUTING.md)
