# VEX Protocol — Roadmap to v1.0

> **Mission**: Researching Accountable Intelligence.
> **Vision**: Building the Cognitive Layer of the ProvnAI "Immune System for AI".

This document outlines the strategic evolution of the VEX protocol following the v0.1.5 milestone.

## [Phase 3] Polish & Hardening (Current Focus)

### Security & Isolation (The Armor)
- **WASM Sandboxing**: Integrate `wasmtime` into `ToolExecutor`. Tools with `Capability::PureComputation` will run in strictly isolated environments to prevent host resource exploitation.
- **Multi-LLM Debate**: [INTEGRATED] Enabled via `vex-router`. heterogeneous model configurations (e.g., Claude vs GPT) for Red/Blue agents to eliminate model-specific bias.
- **Provider Diversity**: [INTEGRATED] Unified `LlmProvider` trait with multi-backend routing logic.
- **Vex-Halt Evaluation**: Integrate with **[Vex-Halt](https://github.com/provnai/vex-halt)** (443+ test items) for automated calibration benchmarking.

### Real-time Interaction & Swarm Connectivity
- **SSE Streaming**: Implement Server-Sent Events (SSE) for the execute endpoint to provide real-time token streams, tool call progress, and audit heartbeats.
- **Prospective Interception Layer**: Research deep integration with **McpVanguard** (Development) for L2-Semantic intent analysis and behavioral monitoring.

### Performance & Settlement
- **Persistent Cache**: [INTEGRATED] Semantic Caching layer added to `vex-router` (Redis/Disk-ready).
- **Settlement Backends**: Optimize anchoring for **ProvnCloud** (Development) and **Notary-AO** (Development).

---

## [Phase 4] Ecosystem & Compute Integration

### Compliance & Registry
- **Dependency Audit**: Full `cargo deny` / `cargo audit` security pass (ISO 42001 compliance).
- **Registry Release**: Official publication of the 11-crate workspace to `crates.io`.

### Compute Pillar Integration
- **VexCore Compute Layer**: Standardize VEX Protocol types for future compatibility with the **VexCore** (Research/Alpha) compute layer (Jolt ZKVM + Nova SNARKs recursive verification).
- **SaaS Settlements**: Finalize bi-directional synchronization with the **ProvnCloud** dashboard settlement layer.

---

## [Phase 5] Future Horizons (v1.1+)
- **Cross-Chain Proof Verification**: Direct On-Chain verification of VEX roots via ZK-Rollups (Plonky3).
- **Swarm Governance**: Epidemic P2P mesh for decentralized model weights diffusion and Proof of Cognition.
