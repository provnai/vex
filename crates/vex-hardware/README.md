# ⚓ vex-hardware (VEX Protocol)

**The silicon-rooted trust anchor for AI agents.**

`vex-hardware` provides the core cryptographic anchoring required for the VEX Protocol's Silicon-Rooted Trust. It enables agents to prove they are running on specific, verified hardware before executing high-stakes tasks.

## 🚀 Key Features

- **Linux TPM 2.0**: Native integration via `tss-esapi` with support for Esys and TCTI loaders.
- **Windows CNG**: Tight integration with the Microsoft Platform Crypto Provider for enterprise-grade security.
- **Deterministic Fallback**: Support for `VEX_HARDWARE_SEED` to maintain stable identities in virtualized cloud environments (Railway, AWS, GCP).
- **Transparent Probing**: Native logging of the hardware discovery process to ensure auditability from the first boot line.

## 🏗 Architecture

This crate is a "Layer 0" dependency in the VEX ecosystem, designed for maximum performance and zero internal VEX dependencies to ensure a clean, portable security boundary.

## ⚖️ License

Apache-2.0 — Part of the [ProvnAI](https://provnai.com) Ecosystem.
