# Changelog

All notable changes to the VEX Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.6] - 2026-02-22

### Added
- **v0.1.6 Ecosystem Synchronization**: Bumped all 13 workspace crates to v0.1.6 to recover from the partial v0.1.5 release and ensure consistent versioning across the registry.

### Changed
- **LlmProvider Migration**: Removed legacy `LlmBackend` references in `vex-runtime` and integration tests in favor of the unified `LlmProvider` trait.
- **Workflow Renaming**: Renamed `vex-cli` to `vex-protocol-cli` in `release.yml` and `Cargo.toml` to resolve the registration conflict on crates.io.

### Fixed
- **CI/CD Stabilization**: Resolved hidden compilation errors in `vex-algoswitch` and `vex-router` benchmarks, examples, and tests discovered by the `--all-targets` CI flag.
- **WSL Green Build**: Verified 100% build compatibility and test pass rates in WSL 2 for all targets and features.
- **AppState Signature**: Fixed signature mismatch in `vex-api` integration tests by correctly initializing the optional `Router` arc.
- **Lint Stabilization**: Refactored `vex-router` gateway with argument structs to resolve Clippy `too_many_arguments` and `redundant_closure` errors.
- **Workspace Cleanup**: Removed redundant imports and applied global `cargo fmt` for a perfectly organized codebase.
- **Benchmark Registry**: Fixed incorrect crate names and missing dev-dependencies (`criterion`) in benchmark configurations.

## [0.1.5] - 2026-02-20

### Added
- **Blockchain Anchoring backends**: Integrated real blockchain providers including Ethereum (EIP-4844 calldata), Celestia (Blobstream), and OpenTimestamps for cryptographic proof of the audit log.
- **Persistent VectorStore**: Replaced in-memory stub with a full SQLite-backed embedding store in `vex-persist`. Added semantic similarity search (cosine similarity) wired into `vex-temporal` for context-aware compression.
- **Real WebSocket MCP Client**: Implemented a functional JSON-RPC over WebSocket client in `vex-llm` with TLS support and async request/response matching.
- **Job Result Retrieval**: Added `GET /api/v1/jobs/{job_id}` endpoint to retrieve persistent execution results from the queue store.
- **WSL Build & Test Suite**: Verified the entire workspace builds and passes 550+ tests in Windows Subsystem for Linux (WSL) for cross-platform reliability.
- **Structured Debate Voting**: Red agent now returns structured JSON (`is_challenge`, `confidence`, `reasoning`, `suggested_revision`) instead of keyword heuristics. Falls back to keyword detection gracefully.
- **LLM Safety Judge**: `sanitize_prompt_async` optionally calls an LLM to perform secondary safety evaluation before any prompt reaches an agent (`use_safety_judge: true` in `SanitizeConfig::prompt()`).
- **Regex-Based Injection Detection**: Replaced `contains()` loop with a compiled `OnceLock<Regex>` covering 30+ injection patterns including 2025-aware adaptive attacks.
- **Audit Types to `vex-core`**: `AuditEvent`, `AuditEventType`, `ActorType`, `HashParams`, `Signature` moved from `vex-persist` to `vex-core/src/audit.rs` for better separation of concerns. Re-exported at crate root.
- **Actor Attribution in Audit Log**: `AuditStore::log()` now accepts `ActorType` and pseudonymizes `Human` actors with SHA-256 (ISO 42001 A.6.2.8 compliance).
- **Ed25519 Signatures**: `Signature::create()` / `Signature::verify()` backed by real `ed25519-dalek` keys for multi-party audit authorization.
- **Intelligent LLM Routing (`vex-router`)**: Integrated a smart routing layer with cost/latency/quality optimization.
  - **Adversarial Role Detection**: Automatically upgrades query quality if system prompts contain "shadow", "adversarial", or "red agent".
  - **Routing Observability**: Comprehensive metrics for cost savings and precision reporting via `GET /api/v1/routing/stats`.
- **AlgoSwitch Optimization (`vex-algoswitch`)**: New optimization crate for dynamic runtime algorithm selection.
  - **Self-Optimizing Merkle Search**: Conditionally switches between recursive and iterative traversal based on data density.
  - **Optimized Audit Hashing**: Adaptive hashing for non-critical event trails to maximize throughput while maintaining ISO 42001 compliance.
- **CLI Tenant Discovery**: `vex verify` now queries SQLite directly (`SELECT DISTINCT tenant_id`) to auto-discover all tenants.

### Changed
- **Blue Agent Confidence**: Debate vote now uses `blue_agent.fitness.max(0.5)` — reports actual evolved fitness rather than a hardcoded 0.8.
- **AppState**: Now holds `Arc<dyn LlmProvider>` to share LLM access with the sanitizer safety judge.
- **`sanitize_prompt` renamed**: Sync variant kept as `sanitize_prompt`, async variant is `sanitize_prompt_async<L: LlmProvider>`.

### Fixed
- Removed erroneous `mount = "0.4"` (Iron web framework) dependency from `vex-llm`.
- Fixed `gen_audit_cli.rs` import path after audit types moved to `vex-core`.

## [0.1.4] - 2025-12-20

### Added
- **Tenant-Scoped Rate Limiting**: Per-tenant limits using the `governor` crate (GCRA algorithm).
  - `TenantRateLimiter` with configurable tiers (Free, Standard, Pro, Unlimited).
  - JWT-based tenant identification with fallback to `x-client-id` header.
- **A2A Protocol Integration**: Full Agent-to-Agent communication suite.
  - Endpoints: `/.well-known/agent.json`, `/a2a/tasks`, `/a2a/tasks/{id}`.
  - Standardized agent capability advertising via agent cards.
  - **NonceCache hardening**: Partial eviction (10%) at 20k entries to prevent memory reset attacks.
- **LLM Resilience & Caching**: Production-grade response optimization.
  - Circuit Breaker pattern for provider failover.
  - `CachedProvider` for response memoization using `moka`.
- **OpenAPI Documentation**: Interactive Swagger UI at `/swagger-ui`.
  - Full `utoipa` integration for all API schemas.
- **HTTPS Enforcement**: Production security requirement with native `tokio-rustls`.
- **Parallel Evolution**: Performance optimization for genome processing via `rayon`.
- **Property-Based Testing**: Added `proptest` for cryptographic primitives verification.
- **Crates.io Readiness**: All 11 crates now have complete metadata (keywords, categories, READMEs).

### Changed
- **BREAKING**: Replaced global `RateLimiter` with `TenantRateLimiter`.
- **BREAKING**: Unified API router signature and updated `axum` to 0.8.
- **Improved Observability**: Injected `request_id` and `tenant_id` into all telemetry spans.
- **Workspace Standardization**: All internal crate dependencies now use `workspace = true`.

### Security
- **Input Sanitization**: Expanded jailbreak patterns for 2025 adaptive attacks (stylistic proxies, audit chain bypass attempts).
- **A2A Replay Protection**: Hardened nonce cache with partial eviction to prevent OOM-based replay windows.
- **Audit Trail Integrity**: Refactored `compute_hash` to use structured parameters (ISO 42001 compliance).

### Fixed
- Middleware JWT claim extraction for tenant identification.
- OpenAPI schema generation errors.
- Axum 0.8 / Hyper 1.0 compatibility issues.
- All Clippy warnings resolved (zero-warning build).

## [0.1.3] - 2025-12-18

### Added
- **Merkle Anchoring**: New `vex-anchor` crate for anchoring audit logs to external providers.
- **Security Hardening**: Implemented 12 critical security remediations (ISO 42001 alignment).
- **Audit Persistence**: Enhanced `AuditStore` with optimized Merkle branch retrieval.
- **Verification CLI**: Enhanced `vex verify` command for deep audit chain inspection.

### Fixed
- Concurrent mutation bugs in the evolution engine.
- Formatting issues in CI pipelines.

## [0.1.2] - 2025-12-18

### Added
- **MCP Tool Suite**: Added built-in tools for `JsonPath`, `Regex`, `Uuid`, `DateTime`, and `Hash`.
- **Evolution Schema**: Introduced SQLite-backed persistence for agent genome generations.
- **Orchestrator Improvements**: Parallelized agent execution using `tokio::spawn`.

## [0.1.1] - 2025-12-17

### Changed
- **Workspace Refactor**: Standardized naming conventions and error handling across all crates.
- **Cleanup**: Removed unused imports and optimized dependencies.

## [0.1.0] - 2025-12-01

### Added
- Initial release of the VEX Protocol.
- Core adversarial verification engine.
- Temporal memory with horizon-based decay.
- Merkle tree-based audit trails.
- JWT Authentication and SQLite persistence.
- Initial support for DeepSeek, Mistral, and OpenAI.
