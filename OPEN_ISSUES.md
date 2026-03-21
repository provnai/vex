# VEX Protocol — Open Issues & Technical Debt

Tracked issues that need resolution before v1.0.

## 🛡️ Security / Stability
- [x] **Missing WASM Sandbox**: [RESOLVED] Implemented `WasmTool` with memory/fuel isolation in v1.3.0.
- [x] **TPM Initialization (0x80280095)**: [RESOLVED] Hardened fallback logic and state recovery implemented in v1.1.4.
- [x] **LLM Timeout Hardening**: [RESOLVED] Implemented `ResilientProvider` with circuit breaker and timeout guards.

## 🧩 Architectural Gaps
- [x] **VectorStore Metadata Search**: [RESOLVED] Implemented JSON-aware SQL filtering in SQLite and Postgres backends.
- [x] **Hardware PCR Binding (v0.2)**: [RESOLVED] Integrated direct TBS interaction for kernel-level integrity measurements (PCRs 0, 7, 11) in v1.3.0.
- [x] **Anchor Error Recovery**: [RESOLVED] Architected as a ProvnCloud (SaaS) responsibility. The open-source `vex-anchor` trait remains lean; gas spikes and retry queues are handled by the cloud settlement layer.
- [x] **Magpie IR Syntax Parity**: [RESOLVED] Fixed double-terminator and signature mismatch issues in v1.3.0.

## ⚡ Performance
- [x] **Async Migration**: [RESOLVED] Decoupled schema migrations from hot-path constructors.
- [x] **Workspace Bloat**: [RESOLVED] Phase 9 cleanup and v1.6.0 "Titan-Grade" sterilization achieved zero-warning/canonical parity.

## 🧪 Post-Release Validation (v1.7.0 Roadmap)
- [ ] **Real-World Benchmarks**: Transition from high-fidelity mocks to live-provider (Claude-3.5-Sonnet/GPT-4o) adversarial stress-testing.
- [x] **JCS Fuzzing**: [RESOLVED] Implemented `jcs_fuzz.rs` property-based testing (5000 iterations passed).
