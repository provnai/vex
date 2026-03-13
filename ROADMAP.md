# VEX Protocol — Roadmap to v1.0

> **Mission**: Researching Accountable Intelligence.
> **Vision**: Building a cognitive infrastructure for verifiable AI reasoning.

This document outlines the strategic evolution of the VEX protocol following the v0.1.5 milestone.

## [Phase 11] Production Hardening & VEP Bundling (Current Focus)

### Security & Isolation (The Armor)
- **WASM Sandboxing**: [COMPLETE] Secure tool execution using `wasmtime` 22.x. Implemented `WasmTool` with memory (64MB) and fuel (10M) limits.
- **Formal Intent Hardening**: [COMPLETE] `MagpieAstBuilder` neutralized IR injection vulnerabilities via programmatic AST generation.
- **OOM Protection**: [COMPLETE] Host-level memory guarding for sandbox outputs.

### Total Truth & Integrity
- **Total Truth CI**: [COMPLETE] Real Magpie compiler integration in CI/CD pipeline.
- **Unified CLI**: [COMPLETE] Standardized Magpie invocation across all VEX pillars.
- **VEP Binary Bundling**: [COMPLETE] Transitioned to TLV-based binary Evidence Capsules with embedded formal ASTs.
- **Hardware PCR Binding (v0.2)**: [COMPLETE] Integrated direct TBS interaction for kernel-level integrity measurements (PCRs 0, 7, 11) on Windows.

### Real-time Interaction & Swarm Connectivity
- **SSE Streaming**: [INTEGRATED] Implement Server-Sent Events (SSE) for the execute endpoint to provide real-time status updates and final results.
- **Prospective Interception Layer**: [INTEGRATED] Deployed `MagpieAstBuilder` and `WSL` interop in `TitanGate` for L2-Semantic intent analysis and behavioral monitoring.
- **George Interop Verification**: [COMPLETE] Verified JCS parity and composite root commitment for v0.2 spec. Drafted "Minimal Commitment" Witness pattern for production.

### Performance & Settlement
- **Persistent Cache**: [COMPLETE] Semantic Caching layer added to `vex-router` (Redis/Disk-ready).
- **CHORA Handshake**: [COMPLETE] Live witness authority decision loop (ALLOW/HALT/ESCALATE).
- **Settlement Backends**: Optimize anchoring for **ProvnCloud** (SaaS) and **Solana** (High-Performance).

---

## [Phase 4] Ecosystem & Compute Integration

### Compliance & Registry
- **Dependency Audit**: Full `cargo deny` / `cargo audit` security pass (ISO 42001 compliance).
- **Registry Release**: Official publication of the 14-crate workspace to `crates.io`.

### Compute Pillar Integration
- **VexCore Compute Layer**: Standardize VEX Protocol types for future compatibility with the **VexCore** (Research/Alpha) compute layer (Jolt ZKVM + Nova SNARKs recursive verification).
- **SaaS Settlements**: Finalize bi-directional synchronization with the **ProvnCloud** dashboard settlement layer.

---

## [Phase 5] Future Horizons (v1.1+)
- **Cross-Chain Proof Verification**: Direct On-Chain verification of VEX roots via ZK-Rollups (Plonky3).
- **Swarm Governance**: Epidemic P2P mesh for decentralized model weights diffusion and Proof of Cognition.
