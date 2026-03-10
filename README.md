# VEX Protocol

> **A protocol for verifiable AI reasoning.**

Adversarial verification • Temporal memory • Cryptographic proofs • Production-oriented API — all in Rust.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/vex-core.svg)](https://crates.io/crates/vex-core)
[![Downloads](https://img.shields.io/crates/d/vex-core.svg?style=flat&color=success)](https://crates.io/crates/vex-core)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/provnai/vex/workflows/CI/badge.svg)](https://github.com/provnai/vex/actions)
[![Docs](https://img.shields.io/badge/docs-provnai.dev-4285F4.svg)](https://www.provnai.dev/docs)
[![Website](https://img.shields.io/badge/website-provnai.com-00C7B7.svg)](https://provnai.com)

**🚀 [Live Demo: AI News Verification on Solana](https://www.vexevolve.com/)** - Experience VEX in action with full transparency and blockchain anchoring.

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/N9-iqS?referralCode=4AXmAG)

📚 **[Railway Deployment Guide →](docs/railway.md)**

---

## Prerequisites

- **Rust 1.75+** with Cargo package manager
- **Go 1.22+** with the standard toolchain (for `attest` CLI)
- **SQLite 3.35+** (for vex-persist & attest - handled automatically)
- **OpenSSL development libraries** (for HTTPS support)
- **TPM 2.0 / Microsoft CNG** (for silicon-rooted identity)
- **Optional**: API keys for LLM providers (DeepSeek, OpenAI, Mistral, Ollama)

📚 **[Full Installation Guide →](https://www.provnai.dev/docs/getting-started)**

---

## Why VEX?

| Problem | VEX Solution |
|---------|--------------|
| **Hallucination** | Red/Blue adversarial debate with consensus |
| **Context Overflow** | Bio-inspired temporal memory with smart decay |
| **Unauditability** | Merkle hash chains with tamper-evident proofs |
| **Rate Limiting** | Tenant-scoped limits with configurable tiers |
| **Agent Isolation** | A2A protocol for secure inter-agent communication |

VEX provides a verification and memory layer designed for production environments that works with any LLM provider.

📚 **[Full Documentation →](https://www.provnai.dev/docs)** | 🔧 **[API Docs (Swagger) →](https://api.provnai.dev/swagger-ui)**

---

## What's New in v1.1.4 🚀

- ⚓ **Hardened TPM Handshakes** - Resolved 0x80280095 initialization errors; physical TPM 2.0 is now performing real-world verifiable signatures in production.
- 📜 **VEP (Verifiable Evidence Packet) v0.1** - Specification locked. The Evidence Capsule is now the protocol's atomic unit of trust.
- 🛡️ **Orchestrator Integration** - End-to-end integration complete. Cognitive intent is now cryptographically bound to hardware identity and witness authority.
- 🧱 **Consistency & Metrics** - Fully audited ~45k line source codebase with 100% compilation parity across Rust and Go.

---

## Quick Start

```bash
# Build
cargo build --workspace --release

# Test (85+ tests)
cargo test --workspace

# Run API server
cargo run --release -p vex-api

# CLI tools
cargo run -p vex-protocol-cli -- tools list
cargo run -p vex-protocol-cli -- tools run calculator '{"expression": "2+2"}'
```

### Environment Variables

```bash
# ── Database ──────────────────────────────────────────────────────────────────
# SQLite (default — single node, Railway volume)
export DATABASE_URL="sqlite:vex.db?mode=rwc"

# PostgreSQL (multi-node — Railway Managed DB, horizontal scaling)
export DATABASE_URL="postgres://user:pass@host/vex"
# On Railway, this is injected automatically when you add a Postgres plugin:
# DATABASE_URL = ${{Postgres.DATABASE_URL}}

# ── LLM Provider (choose one) ─────────────────────────────────────────────────
export DEEPSEEK_API_KEY="sk-..."
# OR: MISTRAL_API_KEY, OPENAI_API_KEY

# ── Auth ──────────────────────────────────────────────────────────────────────
# Generate with: openssl rand -hex 32
export VEX_JWT_SECRET="your-32-character-secret-here"

# ── Rate Limiting (OSS Defaults — override as needed) ─────────────────────────
export VEX_DISABLE_RATE_LIMIT="true"    # Disable entirely (local stress tests)
export VEX_LIMIT_FREE="60"              # req/min for free tier (default: 60)
export VEX_LIMIT_STANDARD="120"         # req/min for standard tier (default: 120)
export VEX_LIMIT_PRO="600"              # req/min for pro tier (default: 600)

# ── Observability (OpenTelemetry) ─────────────────────────────────────────────
# Connect to any OTLP-compatible collector (Grafana, Jaeger, Datadog, etc.)
# Requires building with: cargo build --features vex-api/otel
export OTEL_EXPORTER_OTLP_ENDPOINT="http://your-collector:4317"
export OTEL_SERVICE_NAME="vex-production"
export OTEL_TRACES_SAMPLER_ARG="0.1"    # 10% sampling in production

# ── Production (optional) ─────────────────────────────────────────────────────
export VEX_ENV="production"             # Enforces HTTPS
export VEX_TLS_CERT="/path/to/cert.pem"
export VEX_TLS_KEY="/path/to/key.pem"
```

### Start the Server

```bash
# Development (HTTP)
cargo run --release -p vex-api

# Production (HTTPS enforced)
VEX_ENV=production \
VEX_TLS_CERT=./cert.pem \
VEX_TLS_KEY=./key.pem \
cargo run --release -p vex-api
```

Then visit:
- **API Documentation**: `https://localhost:8080/swagger-ui`
- **Health Check**: `https://localhost:8080/health`
- **Metrics**: `https://localhost:8080/metrics` (Prometheus format)

---




```               
┌─────────────────────────────────────────────────────────────────┐
│  vex-server    │ Production entry point (Railway, Middleware)   │
├────────────────┼────────────────────────────────────────────────┤
│  vex-chora     │ Native Bridge to External Neutral Authority    │
│  vex-api       │ HTTPS API, JWT Auth, Tenant Rate Limiting      │
├────────────────┼────────────────────────────────────────────────┤
│  vex-router    │ Intelligent LLM Routing, Semantic Caching      │
│  vex-llm       │ Providers: DeepSeek, Mistral, OpenAI, Groq    │
│  vex-adversarial│ Red/Blue Debate, Consensus, Reflection       │
├────────────────┼────────────────────────────────────────────────┤
│  vex-runtime   │ Agent Orchestrator, Self-Correcting Genome     │
│  vex-queue     │ Async Worker Pool, Job Processing              │
├────────────────┼────────────────────────────────────────────────┤
│  vex-core      │ Agent, Genome, Merkle Tree, Evolution (Rayon) │
│  vex-hardware  │ Silicon-Rooted Identity (TPM 2.0 / CNG)        │
│  vex-algoswitch│ Runtime Algorithm Selection / Pattern Detect   │
├────────────────┼────────────────────────────────────────────────┤
│  vex-temporal  │ Episodic Memory, 5-Horizon Decay               │
│  vex-persist   │ SQLite, Audit Logs, SQLite Vector Store        │
│  vex-anchor    │ External Merkle Anchoring (File/Git/Ethereum)  │
├────────────────┼────────────────────────────────────────────────┤
│  vex-cli       │ Developer Terminal Interface & Admin Tools     │
│  vex-macros    │ Procedural Codegen for Type-Safe Evolution     │
└────────────────┴────────────────────────────────────────────────┘
```


---

## 🗺️ Trust Flow & Orchestration

VEX Protocol is designed to be the cognitive core of a **Total Trust Trinity**. Depending on your security requirements, it can be deployed in hybrid or full cloud-native configurations.

### **1. The Hardened Cloud-Native Stack**
This flow represents the production gold standard: every intent is silicon-sealed before being verified by neutral authority.

```mermaid
graph LR
    subgraph "External"
        U[User/App]
        T[External API]
    end

    subgraph "ProvnAI Trust Trinity (Cloud)"
        V[VEX - Cognition]
        A[Attest - Identity]
        C[CHORA - Authority]
        G[Vanguard - Security]
    end

    U -->|Request| V
    V -->|Seal Intent| A
    A -->|Hardware Signed| V
    V -->|Governance| C
    C -->|Evidence Capsule| V
    V -->|Audited Action| G
    G -->|Execution| T
    T -->|Result| G
    G -->|Verified Response| U
```

### **2. The Hybrid Sovereign Flow**
Ideal for developers maintaining local control over keys while leveraging cloud-native persistence and neutral authority.

```mermaid
graph LR
    subgraph "Sovereign Enclave (Local)"
        VL[VEX - Cognition]
        AI[Attest - Identity]
    end

    subgraph "Trust Extension (Cloud)"
        VC[CHORA - Authority]
        VP[VEX - Persist]
        VO[OTEL - Metrics]
    end

    VL -->|1. Seal| AI
    AI -->|2. Signed| VL
    VL -->|3. Governance| VC
    VC -->|4. Capsule| VL
    VL -->|5. Sync| VP
    VL -->|6. Telemetry| VO
```

---

## ⚓ Attest (Hardware-Sealed Identity & Provenance)

The [Attest](./attest/README.md) sub-workspace provides the hardware-rooted identity and provenance layer for AI agents. It ensures that an agent's reasoning is cryptographically bound to its physical execution environment.

### **Core Capabilities:**
- **Silicon-Bound Identity**: Keys are sealed to the hardware (TPM 2.0 on Linux, CNG on Windows). The `aid:<pubkey-hash>` ID is deterministically derived from the silicon.
- **Verifiable Execution Receipts**: Every logical decision is signed by the hardware, producing a `.capsule` that proves "the silicon said this."
- **Quantum Undo System**: Automatic filesystem snapshots before every `attest exec` allow for instant state rollback.
- **Policy Guardrails**: Real-time evaluation of agent actions against YAML-based safety rules.

### **Quick Start (CLI):**
```bash
# Build both Rust and Go components
make build

# Initialize the security directory
./attest/attest init

# Create a hardware-sealed identity
./attest/attest agent create --name "my-agent"
```

📚 **[Full Attest Documentation →](./attest/README.md)**

---

## 🛡️ McpVanguard Integration (Cloud-Native Security Proxy)

VEX Protocol natively integrates with **[McpVanguard](https://github.com/provnai/McpVanguard)**, allowing for deep security, correlation, and tool execution proxying across distributed deployments.

By configuring VEX's `vex-llm` module to point to an McpVanguard proxy (instead of executing MCP tools locally), you enable:
- **Centralized Tool Auditing:** All tool executions are logged and cryptographically signed before being routed to external services.
- **Event Correlation:** VEX agent reasoning states and actions uniquely correlate with McpVanguard's network intercepts.
- **Agent Blackboxing:** Isolate the execution context (e.g. database access, heavy computation) from the agent's core reasoning engine.

This is a critical architecture for deploying VEX in "Live Loop" scenarios where actions have real-world consequences (e.g., executing transactions or interacting with external APIs).

---

## Production Features

### 🔐 Security
- **JWT Authentication** with configurable secrets
- **Tenant-Scoped Rate Limiting** (GCRA algorithm via `governor`)
- **HTTPS Enforcement** for production environments
- **Secure Secret Handling** with zeroize

### 📊 Observability
- **OpenAPI 3.0 Specification** (`/api-docs/openapi.json`)
- **Interactive Swagger UI** (`/swagger-ui`)
- **Prometheus Metrics** (`/metrics`)
- **Structured Tracing** with request/tenant IDs

### 🚀 Resilience
- **LLM Circuit Breakers** - Automatic failover on provider issues
- **Response Caching** - Reduces redundant API calls
- **Graceful Degradation** - Fallback to mock provider

### ⚡ Performance
- **Parallel Evolution** - Multi-threaded genome processing
- **Connection Pooling** - HTTP/2 with keep-alive
- **Async-First Design** - Tokio runtime throughout

---

## API Endpoints

| Endpoint | Method | Description |
|----------|--------|-------------|
| `/swagger-ui` | GET | Interactive API documentation |
| `/health` | GET | Basic health check |
| `/health/detailed` | GET | Component-level health status |
| `/.well-known/agent.json` | GET | A2A agent capability card |
| `/a2a/tasks` | POST | Create inter-agent task |
| `/a2a/tasks/{id}` | GET | Query task status |
| `/api/v1/agents` | POST | Create new agent |
| `/api/v1/agents/{id}/execute` | POST | Execute agent with verification |
| `/api/v1/metrics` | GET | JSON metrics |
| `/metrics` | GET | Prometheus metrics |
| `/api/v1/routing/stats` | GET | Real-time routing performance & cost savings |

---

## Testing & Quality

```bash
# Unit + integration tests
cargo test --workspace

# Property-based tests (Merkle trees)
cargo test --package vex-core -- proptests

# Benchmarks (evolution, Merkle)
cargo bench --package vex-core

# LLM integration tests (requires API key)
DEEPSEEK_API_KEY="sk-..." cargo test --package vex-llm -- --ignored
```

---

## Documentation

| Resource | Link |
|----------|------|
| **Full Docs** | [provnai.dev/docs](https://www.provnai.dev/docs) |
| **API Reference (Rustdoc)** | [provnai.dev/rustdoc](https://www.provnai.dev/rustdoc) |
| **API Reference (OpenAPI)** | Run server → `/swagger-ui` |
| **Architecture** | [ARCHITECTURE.md](ARCHITECTURE.md) |
| **Railway Deployment** | [docs/railway-deployment.md](docs/railway-deployment.md) |
| **Roadmap** | [ROADMAP.md](ROADMAP.md) |
| **Open Issues** | [OPEN_ISSUES.md](OPEN_ISSUES.md) |
| **Benchmarks** | [BENCHMARKS.md](BENCHMARKS.md) |
| **Contributing** | [CONTRIBUTING.md](CONTRIBUTING.md) |

---

## 🔗 The ProvnAI Ecosystem
VEX is the central pillar of a multi-layered trust stack designed for the agentic era:

- **1. Identity** ([Provn-SDK](https://github.com/provnai/provn-sdk)): Sovereign Ed25519 signing (no_std).
- **2. Cognition** (VEX Protocol - This repo): Adversarial verification and temporal memory.
- **3. Safety Brake** ([Vex-Halt](https://github.com/provnai/vex-halt)): Emergency circuit breaker and verification benchmark.
- **4. Demonstration** ([VexEvolve](https://www.vexevolve.com)): Production AI newsroom swarm (Live).
- **5. Marketing** ([provnai.com](https://provnai.com)): Global Open Research Initiative portal.
- **6. Developer** ([provnai.dev](https://provnai.dev)): Documentation & Rustdoc portal.
- **7. Authority** (vex-chora): Native bridge to the external CHORA witness network.


---

## License

Apache-2.0 — See [LICENSE](LICENSE)
