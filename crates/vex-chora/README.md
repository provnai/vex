# ⚓ vex-chora (VEX Protocol)

**The Bridge to Neutral Authority.**

`vex-chora` is the native adapter for the external CHORA witness network. It ensures the VEX Protocol remains interoperable with decentralized authority without compromising the privacy of the witness engine.

## 🚀 Key Features

- **JCS Canonicalization**: Native RFC 8785 compliance for cross-platform deterministic hashing.
- **Authority Handshakes**: Standardized traits for requesting and verifying external attestations.
- **Privacy First**: Strictly a client-side bridge; contains no proprietary witness logic or private state.
- **Interoperability**: Designed to allow VEX agents to easily switch between different witness providers.

## 🏗 Architecture

This crate acts as "Layer 1.5"—sitting between `vex-core` (Intent) and the external world. It depends on `vex-core` for basic types but remains independent of the server or API layers.

## ⚖️ License

Apache-2.0 — Part of the [ProvnAI](https://provnai.com) Ecosystem.
