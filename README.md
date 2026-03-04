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

[![Deploy on Railway](https://railway.com/button.svg)](https://railway.com/deploy/N9-iqS?referralCode=4AXmAG)

📚 **[Railway Deployment Guide →](docs/railway.md)**

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

## ✨ What's New in v0.3.0

- � **PostgreSQL & pgvector** - Cloud-native database support with native vector similarity searching via HNSW indexing, enabling horizontal scaling.
- � **Industrial Observability** - Full OpenTelemetry (OTEL) v0.27 integration for exporting agent traces and decisions to any OTLP collector.
- 🛡️ **McpVanguard Integration** - Deep security, correlation, and tool execution proxying across distributed deployments.
- 🚦 **Configurable Rate Limiting** - Flexible GCRA-based API limits via environment variables, giving OSS developers full control over tenant tiers.
- 📦 **Workspace Optimization** - Synchronized dependency structures and robust CI/CD pipelines across all 14 crates.
- � **Railway-Native** - Zero-config deployment templates with 300s healthcheck windows and auto-migrations.

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

---

## License

Apache-2.0 — See [LICENSE](LICENSE)
