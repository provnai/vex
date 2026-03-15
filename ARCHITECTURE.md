# VEX Protocol Architecture

## Overview

VEX is a cognitive framework for building verifiable AI agents. It provides a multi-layered Rust infrastructure for adversarial, temporal, and cryptographically-proven reasoning.

```
┌─────────────────────────────────────────────────────────────┐
│                      Server Layer                           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-server: Production binary entry point (Railway)     ││
│  │             Groq provider, custom middleware              ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                     Gateway Layer                           │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-api: Axum HTTP + JWT + Rate Limiting + Circuit Break││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                   Intelligence Layer                        │
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-llm               │  │ vex-adversarial             ││
│  │ DeepSeek/OpenAI/Groq  │  │ Red/Blue/Reflection Agent   ││
│  └───────────────────────┘  └─────────────────────────────┘│
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-router            │  │ vex-algoswitch              ││
│  │ Intelligent LLM Router│  │ Self-Optimizing Algorithm   ││
│  │ Semantic Cache + Cost │  │ Runtime (pattern detection) ││
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
│                    Hardware Layer                           │
│  ┌───────────────────────┐  ┌─────────────────────────────┐│
│  │ vex-hardware          │  │ vex-sidecar                 ││
│  │ TPM 2.0 / PCR Binding │  │ Silicon Boundary Proxy      ││
│  └───────────────────────┘  └─────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                    Authority Layer                          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-chora: Bridge to neutral witness network             ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                    Interface Layer                          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-cli: Administrative tools & verification             ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                   Persistence Layer                         │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-persist: SQLite + Migrations + Audit Logs          ││
│  │ Semantic VectorStore + VEP Binary Blob Store            ││
│  └─────────────────────────────────────────────────────────┘│
├─────────────────────────────────────────────────────────────┤
│                    Anchoring Layer                          │
│  ┌─────────────────────────────────────────────────────────┐│
│  │ vex-anchor: VEP root anchoring to external systems      ││
│  │ File / Git / OpenTimestamps / Ethereum / Celestia       ││
│  └─────────────────────────────────────────────────────────┘│
└─────────────────────────────────────────────────────────────┘
```

---

## Data Flow

```
                                         ┌──────────────┐
                                         │   vex-cli    │  CLI entry point
                                         │verify / tools│  Audit chain verification,
                                         │    / info    │  tool execution, system info
                                         └──────────────┘
                                                │
         HTTP Request                           │ (reads vex-persist directly)
              │                                 ▼
              ▼                          ┌──────────────┐
┌──────────────┐                         │ vex-persist  │◄─────────────────────┐
│  vex-server  │  Binary entry point     └──────────────┘                      │
│              │  Groq init, middleware                                         │
└──────────────┘                                                                │
      │                                                                         │
      ▼                                                                         │
┌──────────────┐                                                                │
│   vex-api    │  JWT Auth ── Rate Limit ── Circuit Breaker ── A2A             │
└──────────────┘                                                                │
      │                                                                         │
      ▼                                                                         │
┌──────────────┐                                                                │
│  vex-router  │  Intelligent LLM routing ── Semantic Cache ── Guardrails      │
└──────────────┘                                                                │
      │                                                                         │
      ▼                                                                         │
┌──────────────┐                                                                │
│  vex-queue   │  Job enqueued → Async Worker Pool                             │
└──────────────┘                                                                │
      │                                                                         │
      ▼                                                                         │
┌──────────────┐                                                                │
│ vex-runtime  │  Orchestrator creates agent hierarchy                         │
│ (TitanGate)  │  & manages Pillar Bundling (v0.2 Spec)                        │
└──────────────┘                                                                │
      │                                                                         │
      ├──────────────────┬──────────────────┬──────────────────┐                │
      ▼                  ▼                  ▼                  ▼                │
┌──────────────┐  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐          │
│   vex-core   │  │   vex-llm    │  │ vex-hardware │  │  vex-chora   │          │
│ (Agent Logic)│  │ (Reasoning)  │  │ (TPM Binding)│  │ (Witness Br.)│          │
└──────────────┘  └──────────────┘  └──────────────┘  └──────────────┘          │
      │                  │                  │                  │                │
      ▼                  ▼                  ▼                  ▼                │
      └──────────────────┴────────┬─────────┴──────────────────┘                │
                                  │                                             │
                                  ▼                                             │
                           ┌──────────────┐                                     │
                           │ vex-sidecar  │  Silicon Boundary Proxy             │
                           │ (VEP v1.0)   │  Binary VEP Encapsulation (.vep)   │
                           └──────────────┘                                     │
                                  │                                             │
                                  ▼                                             │
                           ┌──────────────┐                                     │
                           │ vex-persist  │  SQLite audit log + VEP blob store ───┘
                           └──────────────┘
                                  │
                                  ▼
                           ┌──────────────┐
                           │  vex-anchor  │  Merkle root anchored externally
                           │              │  File / Git / OTS / Ethereum / Celestia
                           └──────────────┘

Note: vex-macros is compile-time only — #[derive(VexJob)], #[vex_tool],
      #[instrument_agent] — used across crates but not present at runtime.
```

---

## Verifiable Evidence Packet (VEP) v1.0

The **VEP** is the definitive binary wire format for the VEX Protocol. It is designed for high-integrity, language-agnostic persistence and transfer of audit evidence.

### Binary Structure (TLV)

Every `.vep` file/packet follows a **Type-Length-Value** structure for atomic bundling of proofs.

| Offset | Size | Name | Description |
|--------|------|------|-------------|
| 0      | 3    | Magic | `0x56 0x45 0x50` (ASCII "VEP") |
| 3      | 1    | Version | `0x01` |
| 4      | 8    | Length | Full packet length (u64) |
| 12     | 32   | Root | SHA-256 integrity hash of body |
| 44     | Var  | Body | TLV Segments (Capsule, AST, Proofs) |

### TLV Segments

1. **Type `0x01` (Capsule)**: The Joint Capsule (JSON canonicalized via JCS).
2. **Type `0x02` (Intent AST)**: The formal Magpie AST source for the intent.
3. **Type `0x03` (Identity)**: Silicon-rooted measurements (PCRs / AID).
4. **Type `0x04` (Attestation)**: The external witness signature (Ed25519).

---

## Joint Capsule Specification (v0.2)

Developed in collaboration with the **CHORA Witness Network**, the Capsule v0.2 standard ensures bit-identical parity between Rust and Go implementations.

### The Four Pillars

VEX decomposes every agent action into four cryptographically committed pillars:

| Pillar | Responsibility | Hash Surface |
|--------|----------------|--------------|
| **Intent** | *What* was requested? | Rich Magpie AST + Metadata |
| **Authority** | *What* was decided? | Outcome + Reason + Nonce |
| **Identity** | *Who/Where* requested it?| silicon-root (AID) + PCRs |
| **Witness** | *Who* verified it? | Minimal (NodeID + Receipt + TS) |

### Hardening: JCS & Metadata Catch-alls

To prevent structural drift, VEX uses **JSON Canonicalization Scheme (RFC 8785)**. Every pillar struct contains a flattened `metadata` field. This ensures that:
- Unknown extension fields are preserved.
- The binary footprint of every handshake is 100% stable.
- A single-bit flip in any field (even "ignored" ones) causes a root hash failure.

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

### Self-Correcting Evolution (New)

```
┌─────────────────┐
│ Agent Execution │
└────────┬────────┘
         │ Records experiment
         ▼
┌─────────────────┐     ┌──────────────────┐
│ EvolutionMemory │────▶│ Pearson Correlat.│
│ (Episodic)      │     │ (Statistical)    │
└────────┬────────┘     └──────────────────┘
         │ Batch (70+ items)
         ▼
┌─────────────────┐     ┌──────────────────┐
│ ReflectionAgent │────▶│ OptimizationRules│
│ (LLM Analysis)  │     │ (Semantic)       │
└─────────────────┘     └──────────────────┘
                               │
                               ▼ Persistent
                        ┌──────────────────┐
                        │ SQLite           │
                        │ optimization_rules│
                        └──────────────────┘
```

| Component | Purpose |
|-----------|---------|
| EvolutionMemory | Stores experiments with importance decay |
| ReflectionAgent | LLM + statistical analysis for suggestions |
| OptimizationRule | Semantic lessons extracted from experiments |
| EvolutionStore | Persistent storage for cross-session learning |

---

## Security Model

1.  **Authentication**:
    *   JWT tokens with role-based claims (`vex-api`)
    *   Secure secret handling with `zeroize` memory clearing
    *   API keys hashed with **Argon2id** (salted)

2.  **Input Sanitization**:
    *   **50+ Prompt Injection Patterns** blocked (DAN, Policy Puppetry, etc.)
    *   Unicode normalization (homoglyph attack prevention)
    *   JSON depth limiting (DoS prevention)

3.  **Resilience**:
    *   3-state **Circuit Breaker** (Closed → Open → HalfOpen)
    *   **Rate Limiting**: Per-user tier-based limits
    *   **Integer Overflow Checks**: Enabled in release profile

4.  **Audit Trail**:
    *   Cryptographic hash chaining (SHA-256)
    *   Sensitive field redaction (logs sanitized of secrets)
    *   **Hardware-rooted signatures**: Every capsule is silicon-sealed via TPM/CNG.

5.  **Sandboxing & Isolation**:
    *   **Host OOM Protection**: Strict `MAX_WASM_OUTPUT_BYTES` (10MB) limit on sandbox buffers.
    *   **Formal Intent Verification**: Magpie AST Builder replaces string IR construction to prevent instruction injection.

6.  **Network**:
    *   **HSTS** allowed (Strict-Transport-Security)
    *   Strict **CORS** configuration via environment

---

## Tool System (`vex-llm`)

Cryptographically-verified tool execution with Merkle audit integration.

```
┌─────────────────────────────────────────────────────────────┐
│                    Tool Execution Flow                       │
├─────────────────────────────────────────────────────────────┤
│  ToolExecutor                                                │
│  ├── Validate(args)      // Schema + length checks           │
│  ├── Execute(timeout)    // With DoS protection              │
│  ├── WASM Sandbox        // Isolated memory/fuel (Wasmtime)  │
│  ├── OOM Guard           // Strict 10MB output allocation    │
│  ├── Hash(result)        // SHA-256 for Merkle chain         │
│  └── Audit(log)          // To AuditStore                    │
└─────────────────────────────────────────────────────────────┘
```

**Capability System** (for future WASM sandboxing):

| Capability | Description |
|------------|-------------|
| `PureComputation` | No I/O, safe for WASM isolation |
| `Network` | Requires HTTP/WebSocket access |
| `FileSystem` | Requires local file access |
| `Cryptography` | Uses crypto operations |
| `Subprocess` | Can spawn child processes |

---

## MCP Client (`vex-llm`)

Model Context Protocol integration for external tool servers.

```
┌─────────────────────────────────────────────────────────────┐
│                    MCP Client Flow                           │
├─────────────────────────────────────────────────────────────┤
│  McpClient                                                   │
│  ├── Connect(url)        // TLS enforced for remote          │
│  ├── Authenticate        // OAuth 2.1 token                  │
│  ├── ListTools()         // Discover available tools         │
│  └── CallTool(name,args) // Execute with timeout             │
├─────────────────────────────────────────────────────────────┤
│  McpToolAdapter                                              │
│  • Wraps MCP tool as VEX Tool                                │
│  • Results Merkle-hashed for audit                           │
│  • Capability: Network                                       │
└─────────────────────────────────────────────────────────────┘
```

---

## A2A Protocol (`vex-api`)

Agent-to-Agent protocol for cross-framework agent collaboration.

```
┌─────────────────────────────────────────────────────────────┐
│                    A2A Endpoints                             │
├─────────────────────────────────────────────────────────────┤
│  GET  /.well-known/agent.json                                │
│       └── AgentCard { name, skills, auth }                   │
│                                                              │
│  POST /a2a/tasks                                             │
│       └── TaskRequest { skill, input, nonce }                │
│       └── TaskResponse { status, result, merkle_hash }       │
│                                                              │
│  GET  /a2a/tasks/{id}                                        │
│       └── TaskResponse { status, result, merkle_hash }       │
├─────────────────────────────────────────────────────────────┤
│  Security                                                    │
│  • OAuth 2.0 / JWT authentication                            │
│  • Nonce + timestamp replay protection                       │
│  • Task responses include Merkle hash                        │
└─────────────────────────────────────────────────────────────┘
```

---

## Directory Structure

```
vex/
├── crates/
│   ├── vex-server/       # Production binary (Railway entry point, CHORA middleware)
│   ├── vex-api/          # HTTP Server, Auth, Middleware, A2A
│   ├── vex-router/       # Intelligent LLM Router, Semantic Cache, Guardrails
│   ├── vex-llm/          # Providers, Tools, MCP Client, Rate Limit
│   ├── vex-adversarial/  # Shadow, Debate, Consensus
│   ├── vex-algoswitch/   # Self-Optimizing Algorithm Runtime (pattern detection)
│   ├── vex-runtime/      # Orchestrator, Executor
│   ├── vex-queue/        # Worker Pool, Job Trait
│   ├── vex-core/         # Agent, Genome, Merkle, Evolution
│   ├── vex-temporal/     # Memory, Horizon, Compression
│   ├── vex-persist/      # SQLite + Postgres, Stores, Migrations
│   ├── vex-anchor/       # Merkle anchoring (File/Git/OTS/Ethereum/Celestia)
│   ├── vex-hardware/     # Hardware-Rooted Identity (TPM/CNG)
│   ├── vex-chora/        # Authority Bridge to Witness Network
│   ├── vex-sidecar/      # Silicon Boundary Proxy (VEP Encapsulation)
│   ├── vex-cli/          # CLI: tools, verify, info
│   └── vex-macros/       # Procedural Macros
├── examples/
│   └── vex-demo/         # Demo Applications
├── tests/                # Integration Tests
└── scripts/              # Utilities
```
