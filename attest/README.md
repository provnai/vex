# Attest Protocol

**A hardware-rooted identity and provenance protocol for AI agents.**

Hardware identity • ZK-STARK audit trails • Glassbox Provenance • Production-oriented FFI — all in Rust.

[![GitHub Stars](https://img.shields.io/github/stars/provnai/attest?style=flat-square&color=gold)](https://github.com/provnai/attest/stargazers)
[![License](https://img.shields.io/badge/License-Apache_2.0-blue.svg?style=flat-square)](https://opensource.org/licenses/Apache-2.0)
[![CI](https://github.com/provnai/attest/actions/workflows/ci.yml/badge.svg)](https://github.com/provnai/attest/actions)
[![Go Reference](https://img.shields.io/badge/go.dev-reference-007d9c?logo=go&logoColor=white&style=flat-square)](https://pkg.go.dev/github.com/provnai/attest)


[📚 Documentation](https://provnai.dev/docs) | [🔧 Rustdocs](https://provnai.dev/rustdoc) | [💬 Discord](https://discord.gg/provnai)

---

## Prerequisites

*   **Rust 1.75+** with Cargo package manager (for `attest-rs` and `vex-hardware`).
*   **Go 1.21+** with the standard Cobra/Viper toolchain (for the main CLI).
*   **TPM 2.0** (Linux via `tss-esapi`) or **Microsoft CNG** (Windows via `windows-sys`) for hardware-rooted identity.
*   **SQLite 3.35+** (handled automatically via the pure-Go `modernc.org/sqlite` bridge — no CGO required).

---

## Why Attest?

| Problem | Attest Solution |
| :--- | :--- |
| **Spoofed Identity** | Keys are sealed to the silicon (TPM/CNG). The `aid:<sha256-prefix>` ID is deterministically derived from the hardware-bound public key. |
| **Silent Failures** | ZK-STARK proofs (Plonky3 + Goldilocks) mathematically compress entire audit trails into verifiable artifacts. |
| **Unauditable Logic** | Every declared `intent` is cryptographically linked to every `exec` that follows it, forming a tamper-evident chain. |
| **Dangerous Actions** | The policy engine evaluates agent actions in real-time, providing a layer of protection against harmful commands. |
| **Irreversible Mistakes** | The `quantum` system provides state checkpoints so execution can be safely rewound. |

---

## ✨ Recent Improvements (v0.3.1)

*   **JCS Parity Verification**: Confirmed 100% hashing parity with the VEX Protocol reference.
*   **Persistent Safety Policies**: Integrated reloadable YAML-based guardrails for automated command interception.
*   **ZK-STARK Integrity**: Full Plonky3 integration using Goldilocks fields and Two-Adic FRI.
*   **Hardware Isolation**: The `vex-hardware` crate provides key management with memory hygiene.
*   **Cognitive Binding**: Added `with_identity()` support for the VEX Orchestrator to enable hardware-anchored mutation trails.
*   **Portable Storage**: Uses `modernc.org/sqlite` for cross-platform SQLite support without a C compiler.
*   **Quantum System**: State checkpointing with support for diffing and reverting filesystem changes.

---

## Quick Start

```bash
# 1. Build the Rust security core, then the Go CLI
make build

# 2. Run the full test suite (Go + Rust)
make test

# 3. Initialize Attest in your project
attest init

# 4. Create a hardware-sealed agent identity
attest agent create --name "my-agent" --type langchain
```

---

## 🛠️ CLI Reference

### Top-Level Commands

| Command | Description |
| :--- | :--- |
| `attest init` | Initialize the `.attest` security directory and SQLite database. |
| `attest agent` | Manage cryptographic agent identities (Ed25519 keypairs). |
| `attest intent` | Declare the goal ("the why") before an agent executes anything. |
| `attest exec` | Execute a reversible command with automatic state backup. |
| `attest verify` | Verify a cryptographic signature or run a full ZK-STARK verification. |
| `attest policy` | Define and enforce safety rules (allow, warn, block). |
| `attest query` | Query the attestation audit log. |
| `attest git` | Integrate Attest into Git via pre-commit hooks. |
| `attest identity` | View the current hardware identity bound to this machine. |
| `attest hardware` | Seal/Unseal data via TPM/CNG hardware security directly. |
| `attest quantum` | Time-travel checkpoint system for rollback and state diffing. |

### `attest agent` Subcommands

```bash
attest agent create --name "my-agent" --type langchain   # Types: generic, langchain, autogen, crewai, custom
attest agent list                                         # Lists all agents (active + revoked)
attest agent show aid:12345678                            # Show full agent details
attest agent export aid:12345678                          # Export public key
attest agent import /path/to/agent-backup.json           # Restore from export
attest agent delete aid:12345678                          # Revoke an agent permanently
```

### `attest quantum` Subcommands

```bash
attest quantum timeline                          # Visual timeline of all checkpoints
attest quantum diff chk:abc123                   # Compare checkpoint to current state
attest quantum undo chk:abc123                   # Revert filesystem to checkpoint state
attest quantum undo --dry-run chk:abc123         # Preview changes without applying
attest quantum branch chk:abc123 experiment-v1  # Fork a parallel state from a checkpoint
```

### `attest verify` — ZK-STARK Proof

```bash
# Verify a standard Ed25519 signature
attest verify <attestation-id>

# Deep mathematical verification via Plonky3 ZK-STARK
attest verify --zk <attestation-id>
```

---

## ⚙️ Environment Variables

All environment variables use the `ATTEST_` prefix (set automatically via Viper).

| Variable | Default | Description |
| :--- | :--- | :--- |
| `ATTEST_DATA_DIR` | `~/.attest` | Path to the local security and SQLite storage directory. |
| `ATTEST_LOG_LEVEL` | `info` | Logging verbosity: `debug`, `info`, `warn`, `error`. |
| `DATABASE_URL` | `~/.attest/attest.db` | Override the DB path (SQLite or Postgres URI). |

---

The primary way to use Attest is by anchoring a VEX agent to a hardware-root identity. This creates a verifiable audit trail for cognitive cycles.

```rust
use std::sync::Arc;
use vex_hardware::{HardwareKeystore, AgentIdentity};
use vex_runtime::Orchestrator;
use vex_persist::AuditStore;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Initialize connection to the hardware (TPM2 on Linux, CNG on Windows).
    //    Falls back to software if VEX_HARDWARE_ATTESTATION != "true".
    let keystore = HardwareKeystore::new().await?;

    // 2. Seal your agent's Ed25519 seed to the hardware chip once.
    //    Store `sealed_blob` in your persistence layer (e.g. AuditStore).
    let seed: [u8; 32] = /* load or generate your seed */ [0u8; 32];
    let sealed_blob = keystore.seal_identity(&seed).await?;

    // 3. Unseal the identity for real-time signing from the stored blob.
    let identity: Arc<AgentIdentity> = Arc::new(
        keystore.get_identity(&sealed_blob).await?
    );
    // identity.agent_id = "<uuid derived from ed25519 pubkey via SHA-256>"

    // 4. Bind identity to the VEX Orchestrator.
    //    Every action and evolution is now hardware-signed and ZK-provable.
    let orchestrator = Orchestrator::new(llm_provider, memory, None)
        .with_identity(identity, Arc::new(AuditStore::new(backend)));

    Ok(())
}
```


---

## Testing & Quality

```bash
# Full test suite (Go with race detection + Rust)
make test

# Rust ZK-STARK unit tests in release mode
cd attest-rs && cargo test --release

# Clippy — zero warnings enforced
cd attest-rs && cargo clippy --all-targets -- -D warnings
```

---

## 📐 Architecture

```text
┌─────────────────────────────────────────────────────────────────┐
│  Attest CLI    │ System-level management (Go + Cobra)           │
│                │ agent, intent, exec, policy, quantum, git...   │
├────────────────┼────────────────────────────────────────────────┤
│  pkg/bridge    │ FFI Layer — CGO-free SQLite bridge             │
│  pkg/storage   │ SQLite DB migrations, audit log storage        │
│  pkg/guardrails│ Checkpoint management (quantum undo system)    │
├────────────────┼────────────────────────────────────────────────┤
│  attest-rs     │ Plonky3 ZK-STARK Prover, AuditAir constraints  │
│                │ FRI hardening, forgery-proof verification.     │
├────────────────┼────────────────────────────────────────────────┤
│  vex-hardware  │ TPM2 (Linux) and CNG (Windows) key synthesis,  │
│                │ Ed25519 signing, Zeroize memory hygiene.       │
├────────────────┼────────────────────────────────────────────────┤
│  sdk/python    │ Native Python client + LangChain callbacks     │
│  sdk/js        │ TypeScript-first Node.js client                │
└────────────────┴────────────────────────────────────────────────┘
```

[📐 Full Architecture Document →](docs/architecture.md)

---

## 🛡️ Production Features

### 🔐 Security & Isolation
*   **TPM2/CNG Binding**: Private keys are hardware-sealed and never exposed to the host OS in plaintext.
*   **Memory Zeroization**: Strict use of the `Zeroize` trait for all cryptographic material at drop.
*   **JCS Deterministic Signing**: RFC 8785 serialization ensures identical signatures across heterogeneous systems.
*   **Ed25519 Identities**: Agent IDs are derived as `aid:ed25519:<hex_pubkey>` — uniquely and deterministically addressable, with a 1:1 mapping to VEX agent UUIDs via SHA-256.

### ⚡ Performance
*   **Async-First**: Non-blocking I/O for all hardware and database operations (Tokio runtime).
*   **Optimized STARKs**: Goldilocks-based field arithmetic for millisecond-range proof verification.
*   **FFI Efficiency**: Direct memory mapping between Go CLI and Rust backend with minimal overhead.

### 🚀 Resilience
*   **CGO-Free Storage**: Absolute portability via `modernc.org/sqlite` — no C compiler required.
*   **Graceful Fallback**: Automatic software-signing fallback if TEE hardware is absent.
*   **Reversible Execution**: Automatic state snapshots before every `exec` with hash-verified restoration.

---

## 🔗 The ProvnAI Ecosystem

Attest is the foundational anchor of a multi-layered trust stack designed for the agentic era:

- **1. Identity** (Attest Protocol - This repo): Hardware identity + ZK-STARK audit trails.
- **2. Cognition** ([VEX Protocol](https://github.com/provnai/vex)): Adversarial verification and temporal memory.
- **3. Safety Brake** ([Vex-Halt](https://github.com/provnai/vex-halt)): Emergency circuit breaker and verification benchmark.
- **4. Governance** ([McpVanguard](https://github.com/provnai/mcp-vanguard)): Distributed security proxy and guardrail enforcement.
- **5. Demonstration** ([VexEvolve](https://www.vexevolve.com)): Production AI newsroom swarm (Live).
- **6. Developer** ([provnai.dev](https://provnai.dev)): Documentation & Rustdoc portal.

---

## License

Apache-2.0 — See [LICENSE](LICENSE)
