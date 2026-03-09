# vex-sidecar

<div align="center">
  <img src="https://github.com/provnai/vex/raw/main/.github/assets/vex_logo.png" alt="VEX Logo" width="200" />
</div>

<div align="center">
  <strong>VEX-Sidecar: Transparent Interception Proxy for the VEX Protocol</strong>
</div>

<br />

`vex-sidecar` is a high-performance interception proxy designed to bridge legacy AI agents and services into the **VEX (Verifiable Entity Execution)** trust ecosystem. It encapsulates raw HTTP/LLM traffic into cryptographically-signed **VEP (Viking Enveloped Packets)**, providing mathematical proof of intent, authority, and hardware-rooting without requiring manual integration into black-box systems.

## 🚀 Overview

In a VEX-hardened environment, every agent action must be verifiable. Legacy systems often lack the native "Silicon Boundary" logic required to generate hardware-anchored proofs. `vex-sidecar` solves this by acting as a "trust gateway":

1.  **Intercepts** standard HTTP/REST requests from legacy agents.
2.  **Analyzes** the intent and generates a `ContextPacket`.
3.  **Encapsulates** the payload into a binary VEP envelope.
4.  **Routes** the verifiable packet through the VEX hardware layer (TPM/Secure Enclave) for signing.
5.  **Forwards** the completed, verifiable capsule to the desired endpoint.

## ✨ Key Features

- **Zero-Code Integration:** Bring existing agents into the trust trinity without changing a single line of legacy code.
- **Protocol Encapsulation:** Native support for the VEP v2 binary format and CHORA Capsule Protocol v1.
- **Automatic Provenance:** Automatically attaches hardware-rooted identity and execution metadata to every request.
- **Asynchronous & High Performance:** Built on `axum` and `tokio` for minimal latency impact.

## 🛠 Usage

Set the following environment variables to configure the proxy:

- `VEX_API_URL`: The endpoint of your local VEX control plane (default: `http://localhost:8000`).
- `VEX_TARGET_URL`: The upstream service that processes VEP packets (default: `http://localhost:3000/v2/vep`).

Then simply point your legacy agent's API calls to the sidecar address (default: `http://localhost:8080`).

## 🧱 Part of the VEX Trinity

`vex-sidecar` works in harmony with the core VEX stack:
- [`vex-core`](../vex-core) - Cryptographic primitives and Merkle types.
- [`vex-hardware`](../vex-hardware) - TPM and Secure Silicon integration.
- [`vex-runtime`](../vex-runtime) - Policy enforcement and verification.

## ⚖️ License

Licensed under either of Apache License, Version 2.0 or MIT license at your option.
