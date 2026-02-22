# VEX Protocol — Open Issues & Technical Debt

Tracked issues that need resolution before v1.0.

## 🛡️ Security / Stability
- [ ] **Missing WASM Sandbox**: `ToolExecutor` calculates capabilities but executes all tools in-process. This is a high-priority gap for production environments requiring strict isolation.
- [ ] **LLM Timeout Hardening**: Improve HTTP-level cancellation for unresponsive providers.
- [ ] **Audit Hash Integrity**: Verify Merkle implementation against modern hash-chain attack vectors.
- [ ] **Dead Code Cleanup**: Resolve remaining compiler warnings in `vex-llm` and `vex-adversarial`.

## 🧩 Architectural Gaps
- [ ] **Adversarial Schema Fallback**: Move from string-keyword heuristics to a forced JSON-retry loop for Red agent votes.
- [ ] **VectorStore Filtering**: Support SQL-based metadata filtering in the embedding search.
- [ ] **Anchor Retry Mechanism**: Implement a persistent retry queue for failed blockchain anchor attempts.

## ⚡ Performance
- [ ] **Async Migration**: Move database schema migrations out of the hot path constructor.
- [ ] **Workspace Bloat**: Optimize internal dependencies to reduce binary size.

## 🧪 Testing Gaps
- [ ] **Real-World Benchmarks**: Integration tests using live Claude/GPT models (currently mostly mocked).
- [ ] **Concurrency Stress**: Parallel agent orchestration handle contention testing (SQLite).
