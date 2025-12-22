# VEX Protocol

> **The trust layer for AI agents.**

Adversarial verification • Temporal memory • Cryptographic proofs • Production-ready API — all in Rust.

[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)
[![Crates.io](https://img.shields.io/crates/v/vex-core.svg)](https://crates.io/crates/vex-core)
[![Rust](https://img.shields.io/badge/Rust-1.75+-orange.svg)](https://www.rust-lang.org/)
[![CI](https://github.com/provnai/vex/workflows/CI/badge.svg)](https://github.com/provnai/vex/actions)
[![Docs](https://img.shields.io/badge/docs-provnai.dev-4285F4.svg)](https://www.provnai.dev/docs)
[![Website](https://img.shields.io/badge/website-provnai.com-00C7B7.svg)](https://provnai.com)

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

## ✨ What's New in v0.1.4

- 🔒 **Tenant-Scoped Rate Limiting** - Per-tenant quotas with JWT-based enforcement
- 🤝 **A2A Protocol** - Agent-to-Agent communication with task queueing
- 💾 **LLM Caching & Circuit Breakers** - Response caching + fault tolerance
- 📖 **OpenAPI Documentation** - Auto-generated Swagger UI at `/swagger-ui`
- 🔐 **HTTPS Enforcement** - Production mode requires TLS certificates
- ⚡ **Parallel Evolution** - Multi-threaded genome processing with rayon
- ✅ **Property-Based Testing** - Proptest coverage for cryptographic primitives

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
│  vex-api       │ HTTPS API, JWT Auth, Tenant Rate Limiting      │
│                │ OpenAPI Docs, A2A Protocol, Swagger UI         │
├────────────────┼────────────────────────────────────────────────┤
│  vex-llm       │ Providers: DeepSeek, Mistral, OpenAI, Ollama  │
│                │ Caching + Circuit Breakers + 6 Built-in Tools  │
│  vex-adversarial│ Red/Blue Debate, Consensus, Reflection       │
├────────────────┼────────────────────────────────────────────────┤
│  vex-runtime   │ Agent Orchestrator, Self-Correcting Genome     │
│  vex-queue     │ Async Worker Pool, Job Processing              │
├────────────────┼────────────────────────────────────────────────┤
│  vex-core      │ Agent, Genome, Merkle Tree, Evolution (Rayon) │
│  vex-temporal  │ Episodic Memory, 5-Horizon Decay               │
├────────────────┼────────────────────────────────────────────────┤
│  vex-persist   │ SQLite, Audit Logs, Merkle Hash Chains         │
│  vex-anchor    │ Blockchain Anchoring (Optional)                │
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
| **Benchmarks** | [BENCHMARKS.md](BENCHMARKS.md) |
| **Contributing** | [CONTRIBUTING.md](CONTRIBUTING.md) |

---

## License

MIT — See [LICENSE](LICENSE)
