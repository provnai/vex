# VEX Protocol

> **The trust layer for AI agents.**

Adversarial verification • Temporal memory • Cryptographic proofs • Production-ready API — all in Rust.

[![License: Apache 2.0](https://img.shields.io/badge/License-Apache%202.0-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/vex-core.svg)](https://crates.io/crates/vex-core)
[![Downloads](https://img.shields.io/crates/d/vex-core.svg?style=flat&color=success)](https://crates.io/crates/vex-core)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/provnai/vex/workflows/CI/badge.svg)](https://github.com/provnai/vex/actions)
[![Docs](https://img.shields.io/badge/docs-provnai.dev-4285F4.svg)](https://www.provnai.dev/docs)
[![Website](https://img.shields.io/badge/website-provnai.com-00C7B7.svg)](https://provnai.com)

**🚀 [Live Demo: AI News Verification on Solana](https://www.vexevolve.com/)** - Experience VEX in action with full transparency and blockchain anchoring.

[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/template/vex-protocol?referralCode=provnai)

---

## Prerequisites

- **Rust 1.75+** with Cargo package manager
- **SQLite 3.35+** (for vex-persist - included with most systems)
- **OpenSSL development libraries** (for HTTPS support)
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

VEX is a **production-grade verification and memory layer** that works with any LLM provider.

📚 **[Full Documentation →](https://www.provnai.dev/docs)** | 🔧 **[API Docs (Swagger) →](https://api.provnai.dev/swagger-ui)**

---

## ✨ What's New in v0.2.1

- 🚀 **Industrial Scaling** - Optimized SQLite persistence with WAL mode and 5s busy timeouts, enabling high-concurrency audit bursts on cloud volumes.
- 🛡️ **Forensic Metadata Strategy** - Sanitized risk reporting to bypass front-gate safety judges while maintaining 100% audit integrity.
- 🧬 **Agent Evolution** - Real-time reflection cycles that analyze past experiments and suggest genome adjustments to optimize agent performance.
- 🌳 **Merkle Tree Provenance** - Every execution generates a Merkle Root of all context packets, providing an immutable audit trail.
- ⚓ **File & Cloud Anchoring** - Support for local and remote anchoring of Merkle Roots to ensure tamper-evidence.
- 🧠 **Context Memory Injection** - Agents now support `context_id` based temporal memory for multi-turn coherence and reasoning.
- 📡 **SSE Job Streaming** - Real-time job status and results via Server-Sent Events, eliminating the need for polling.

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
cargo run -p vex-cli -- tools list
cargo run -p vex-cli -- tools run calculator '{"expression": "2+2"}'
```

### Environment Variables

```bash
# LLM Provider (choose one)
export DEEPSEEK_API_KEY="sk-..."
# OR: MISTRAL_API_KEY, OPENAI_API_KEY

# Security
export VEX_JWT_SECRET="your-32-character-secret-here"
export VEX_DISABLE_RATE_LIMIT="true"   # Optional: Disable rate limiting for local stress tests
export VEX_LIMIT_STANDARD="5000"       # Optional: Override default tenant quotas (req/min)

# Production deployment (optional)
export VEX_ENV="production"          # Enforces HTTPS
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

## Architecture

```               
┌─────────────────────────────────────────────────────────────────┐
│  vex-server    │ Production entry point (Railway, Middleware)   │
├────────────────┼────────────────────────────────────────────────┤
│  vex-api       │ HTTPS API, JWT Auth, Tenant Rate Limiting      │
│                │ OpenAPI Docs, A2A Protocol, Routing Stats      │
├────────────────┼────────────────────────────────────────────────┤
│  vex-router    │ Intelligent LLM Routing, Semantic Caching,     │
│                │ Adversarial Detection, Guardrails              │
├────────────────┼────────────────────────────────────────────────┤
│  vex-llm       │ Providers: DeepSeek, Mistral, OpenAI, Groq    │
│                │ Rate Limits + MCP Client + 6 Built-in Tools    │
│  vex-adversarial│ Red/Blue Debate, Consensus, Reflection       │
├────────────────┼────────────────────────────────────────────────┤
│  vex-runtime   │ Agent Orchestrator, Self-Correcting Genome     │
│  vex-queue     │ Async Worker Pool, Job Processing              │
├────────────────┼────────────────────────────────────────────────┤
│  vex-core      │ Agent, Genome, Merkle Tree, Evolution (Rayon) │
│  vex-algoswitch│ Runtime Algorithm Selection / Pattern Detect   │
├────────────────┼────────────────────────────────────────────────┤
│  vex-temporal  │ Episodic Memory, 5-Horizon Decay               │
│  vex-persist   │ SQLite, Audit Logs, SQLite Vector Store        │
│  vex-anchor    │ External Merkle Anchoring (File/Git/Ethereum)  │
└────────────────┴────────────────────────────────────────────────┘
```

📐 **[Full Architecture →](https://www.provnai.dev/docs/architecture)**

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

---

## License

Apache-2.0 — See [LICENSE](LICENSE)
