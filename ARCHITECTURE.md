# VEX Protocol Architecture

## Overview

VEX (Verified Evolutionary Xenogenesis) is a multi-layered Rust framework for building adversarial, temporal, cryptographically-verified AI agents.

```
┌─────────────────────────────────────────────────────────────┐
│                     Gateway Layer                           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-api: Axum HTTP + JWT + Rate Limiting + Circuit Break││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                   Intelligence Layer                        │
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-llm               │  │ vex-adversarial             ││
│  │ DeepSeek/OpenAI/Ollama│  │ Red/Blue Debate Engine      ││
│  └───────────────────────┘  └─────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                    Execution Layer                          │
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-runtime           │  │ vex-queue                   ││
│  │ Orchestrator + Executor│  │ Async Worker Pool          ││
│  └───────────────────────┘  └─────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                      Core Layer                             │
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-core              │  │ vex-temporal                ││
│  │ Agent + Genome + Merkle│  │ Episodic Memory + Decay    ││
│  └───────────────────────┘  └─────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                   Persistence Layer                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-persist: SQLite + Migrations + Audit Logs          ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

```
User Request
     │
     ▼
┌─────────────┐
│  vex-api    │──── JWT Auth ──── Rate Limit ──── Circuit Breaker
└─────────────┘
     │
     ▼
┌─────────────┐
│ vex-runtime │ Orchestrator creates agent hierarchy
└─────────────┘
     │
     ├──────────────────────────────────┐
     ▼                                  ▼
┌─────────────┐                  ┌─────────────┐
│ Blue Agent  │◄───── Debate ────│ Red Shadow  │
│ (Primary)   │                  │ (Challenger)│
└─────────────┘                  └─────────────┘
     │                                  │
     ▼                                  ▼
┌─────────────┐                  ┌─────────────┐
│  vex-llm    │                  │ Pattern     │
│  Provider   │                  │ Heuristics  │
└─────────────┘                  └─────────────┘
     │                                  │
     └──────────────┬───────────────────┘
                    ▼
             ┌─────────────┐
             │  Consensus  │ Voting Protocol
             └─────────────┘
                    │
                    ▼
             ┌─────────────┐
             │ Merkle Tree │ Hash Chain
             └─────────────┘
                    │
                    ▼
             ┌─────────────┐
             │ vex-persist │ Audit Log
             └─────────────┘
```

---

## Key Components

### Agent (`vex-core`)

```rust
Agent {
    id: Uuid,
    parent_id: Option<Uuid>,
    config: AgentConfig,
    genome: Genome,
    generation: u32,
    fitness: f64,
}
```

### Genome

Five behavioral traits that map to LLM parameters:

| Trait | LLM Parameter | Range |
|-------|---------------|-------|
| exploration | temperature | 0.1 - 1.5 |
| precision | top_p | 0.5 - 1.0 |
| creativity | presence_penalty | 0.0 - 1.0 |
| skepticism | frequency_penalty | 0.0 - 0.5 |
| verbosity | max_tokens | 0.5x - 2.0x |

### Consensus Protocols

| Protocol | Threshold | Use Case |
|----------|-----------|----------|
| Majority | >50% | Quick decisions |
| SuperMajority | ≥67% | Important decisions |
| Unanimous | 100% | Critical decisions |
| WeightedConfidence | Weighted avg ≥0.7 | Nuanced decisions |

### Memory Horizons

| Horizon | Duration | max_entries |
|---------|----------|-------------|
| Immediate | 5 min | 10 |
| ShortTerm | 1 hour | 25 |
| MediumTerm | 24 hours | 50 |
| LongTerm | 1 week | 100 |
| Permanent | ∞ | 500 |

---

## Security Model

1. **Authentication**: JWT tokens with role-based claims
2. **Rate Limiting**: Per-user tier-based limits
3. **Input Sanitization**: Prompt injection detection (17 patterns)
4. **Circuit Breaker**: 3-state FSM prevents cascade failures
5. **Audit Trail**: Hash-chained events with Merkle proofs
6. **Tenant Isolation**: User-prefixed storage keys

---

## Directory Structure

```
vex/
├── crates/
│   ├── vex-core/         # Agent, Genome, Merkle, Evolution
│   ├── vex-adversarial/  # Shadow, Debate, Consensus
│   ├── vex-temporal/     # Memory, Horizon, Compression
│   ├── vex-persist/      # SQLite, Stores, Migrations
│   ├── vex-api/          # HTTP Server, Auth, Middleware
│   ├── vex-runtime/      # Orchestrator, Executor
│   ├── vex-queue/        # Worker Pool, Job Trait
│   ├── vex-llm/          # Providers, Rate Limit, Metrics
│   └── vex-macros/       # Procedural Macros
├── examples/
│   └── vex-demo/         # Demo Applications
├── tests/                # Integration Tests
└── scripts/              # Utilities
```
