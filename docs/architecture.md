# VEX Architecture

For the full architecture documentation, see [ARCHITECTURE.md](../ARCHITECTURE.md) in the repository root.

## System Overview

```
┌─────────────────────────────────────────────────────────────┐
│                     Gateway Layer                           │
│  vex-api: Axum HTTP + JWT + Rate Limiting + Circuit Breaker │
├─────────────────────────────────────────────────────────────┤
│                   Intelligence Layer                        │
│  vex-llm              │  vex-adversarial                    │
│  DeepSeek/OpenAI      │  Red/Blue Debate Engine             │
├─────────────────────────────────────────────────────────────┤
│                    Execution Layer                          │
│  vex-runtime          │  vex-queue                          │
│  Orchestrator/Executor│  Async Worker Pool                  │
├─────────────────────────────────────────────────────────────┤
│                      Core Layer                             │
│  vex-core             │  vex-temporal                       │
│  Agent/Genome/Merkle  │  Episodic Memory + Decay            │
├─────────────────────────────────────────────────────────────┤
│                   Persistence Layer                         │
│  vex-persist: SQLite + Migrations + Audit Logs              │
└─────────────────────────────────────────────────────────────┘
```

## Key Concepts

### Fractal Agents

Agents are hierarchical and can spawn child agents for subtasks.

### Adversarial Verification

Every agent can have a "shadow" that challenges its outputs using Red/Blue debate.

### Temporal Memory

Bio-inspired memory with automatic decay and LLM-powered compression.

### Merkle Proofs

All agent decisions are hash-chained for tamper-evident audit trails.

## Learn More

- [API Reference](https://provnai.dev/vex_core/)
- [Benchmarks](../BENCHMARKS.md)
