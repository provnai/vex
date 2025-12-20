# Changelog

All notable changes to the VEX Protocol will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.1.4] - 2025-12-20

### Added
- **Tenant-Scoped Rate Limiting**: Per-tenant limits using the `governor` crate.
  - `TenantRateLimiter` with configurable tiers (Standard, Premium, Enterprise).
  - JWT-based tenant identification with fallback to `x-client-id` header.
- **A2A Protocol Integration**: Full Agent-to-Agent communication suite.
  - Endpoints: `/.well-known/agent.json`, `/a2a/tasks`, `/a2a/tasks/{id}`.
  - Standardized agent capability advertising via agent cards.
- **LLM Resilience & Caching**: Production-grade response optimization.
  - Circuit Breaker pattern for provider failover.
  - `CachedProvider` for response memoization using `moka`.
- **OpenAPI Documentation**: Interactive Swagger UI at `/swagger-ui`.
  - Full `utoipa` integration for all API schemas.
- **HTTPS Enforcement**: Production security requirement with native `tokio-rustls`.
- **Parallel Evolution**: Performance optimization for genome processing via `rayon`.
- **Property-Based Testing**: Added `proptest` for cryptographic primitives verification.

### Changed
- **BREAKING**: Replaced global `RateLimiter` with `TenantRateLimiter`.
- **BREAKING**: Unified API router signature and updated `axum` to 0.8.
- **Improved Observability**: Injected `request_id` and `tenant_id` into all telemetry spans.

### Fixed
- Middleware JWT claim extraction for tenant identification.
- OpenAPI schema generation errors.
- Axum 0.8 / Hyper 1.0 compatibility issues.

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
