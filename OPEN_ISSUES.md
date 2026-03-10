# VEX Protocol — Open Issues & Technical Debt

Tracked issues that need resolution before v1.0.

## 🛡️ Security / Stability
- [ ] **Missing WASM Sandbox**: `ToolExecutor` calculates capabilities but executes all tools in-process. This is a high-priority gap for production environments requiring strict isolation.
- [x] **TPM Initialization (0x80280095)**: [RESOLVED] Hardened fallback logic and state recovery implemented in v1.1.4.
- [ ] **LLM Timeout Hardening**: Improve HTTP-level cancellation for unresponsive providers.

## 🧩 Architectural Gaps
- [ ] **VectorStore Metadata Search**: Current SQLite implementation searching on vector similarity only; need SQL filters on the metadata JSON field.
- [ ] **Anchor Error Recovery**: [PARTIAL] Trait-based anchoring implemented in v0.2.0 (`vex-anchor`), but still needs a robust retry queue for blockchain gas price spikes.

## ⚡ Performance
- [ ] **Async Migration**: Move database schema migrations out of the hot path constructor.
- [ ] **Workspace Bloat**: Optimize internal dependencies to reduce binary size.

## 🧪 Testing Gaps
- [ ] **Real-World Benchmarks**: Integration tests using live Claude/GPT models (currently mostly mocked).
- [ ] **Coverage Depth**: While all 45k lines compile and pass integration, we need deeper fuzzing for the JCS canonicalization layer.
