# Attest — Hardware-Rooted Privacy & Identity

> **Silicon-rooted trust for AI agents.**

Attest is the identity and provenance layer of the VEX Protocol. It ensures that an agent's reasoning is cryptographically bound to its physical execution environment.

## Overview

Attest provides:
- **Silicon-Bound Identity**: Keys are sealed to the hardware (TPM 2.0 / CNG).
- **Verifiable Execution Receipts**: Signs every logical decision, producing a `.capsule`.
- **Policy Guardrails**: Real-time evaluation of actions against YAML-based safety rules.

## Documentation
- [Architecture Overview](./docs/architecture.md)
- [Rust SDK (attest-rs)](./attest-rs/README.md)
