# VEX Protocol — Open Issues & Technical Debt

Tracked issues that need resolution before v1.0.

## 🛡️ Security / Stability
- [ ] **Missing WASM Sandbox**: `ToolExecutor` calculates capabilities but executes all tools in-process. This is a high-priority gap for production environments requiring strict isolation.
- [ ] **LLM Timeout Hardening**: Improve HTTP-level cancellation for unresponsive providers.
- [ ] **Audit Hash Collision Proofing**: Current Merkle implementation uses SHA-256. Need to verify leaf structure against length-extension attacks.
- [x] **Verified Stability (v0.1.7)**: Replaced critical path `unwrap()` calls and resolved all compiler lints in `vex-api` and `vex-llm`.

## 🧩 Architectural Gaps
- [x] **Adversarial Reliability**: Red Agent now returns structured JSON; Blue Agent enters a "Reflection" phase to eliminate agreement bias.
- [ ] **VectorStore Metadata Search**: Current SQLite implementation searching on vector similarity only; need SQL filters on the metadata JSON field.
- [ ] **Anchor Error Recovery**: If a blockchain anchor fails (e.g., gas price spike), the orchestrator currently just logs a warning. Needs a retry queue.

## ⚡ Performance
- [ ] **Async Migration**: Move database schema migrations out of the hot path constructor.
- [ ] **Workspace Bloat**: Optimize internal dependencies to reduce binary size.

## 🧪 Testing Gaps
- [ ] **Real-World Benchmarks**: Integration tests using live Claude/GPT models (currently mostly mocked).
- [ ] **Concurrency Stress**: Parallel agent orchestration handle contention testing (SQLite).
