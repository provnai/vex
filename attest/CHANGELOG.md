# Changelog

All notable changes to Attest are documented in this file.
The project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [v0.3.1] - 2026-03-10

### Added

- **🧬 Cross-Implementation Verification**: Confirmed 100% JCS hashing parity with the VEX Protocol reference.
- **🛡️ Persistent Guardrails**: Refactored the execution engine to support persistent, reloadable YAML safety policies.

## [v0.3.0] - 2026-03-08

### Added

- **🤝 Capsule V1 Alignment**: Updated capsule root calculation to use JCS-based composite hashing.
- **🏗️ VEP Refactor**: Refactored the internal segment structure to promote Witness and Signature data to top-level pillars.
- **⚡ Noise State Machine**: Implemented Noise_XX handshake for authenticated session establishment.
- **⚓ TPM Key Binding**: Linked session security states to hardware-sealed identities.
- **🕵️ Debug Support**: Added optional encryption toggle for VEP construction during parity testing.
- **🐧 WSL Support**: Added platform-specific stub for TPM interfaces in virtual environments.


## [v0.2.0] - 2026-03-07

### Added
- **TPM Integrity Layer**: Added checksum verification to hardware-sealed identities to prevent data corruption.
- **Hardware Resilience**: Verified driver stability under concurrent load and improved recovery for lost keys.
- **Improved Error Messaging**: Mapped system-level hardware codes to descriptive messages.
- **Policy Utility**: Implemented `attest policy check` for manual verification of command strings.
- **Automation Support**: Added `--passphrase` flag for non-interactive certificate and agent creation.

### Fixed
- **Persistence**: Fixed an issue where guardrail settings were not saved between sessions.
- **Database Schema**: Implemented automated migrations for consistent schema updates.
- **Display**: Corrected Intent ID truncation in CLI list views.
- **Backup Logic**: Improved automated backup handling for directory-level operations.
- **Go/Rust Bridge**: Resolved protocol issues related to RSA padding and buffer lengths.


## [v0.1.0] - 2026-03-05

### ⚓ The Silicon-Rooted Release
This release implements hardware-rooted identity and verifiable audit trails.

### Added
- **Hardware-Sealed Identity**: 
  - Native TPM 2.0 (Linux) and CNG (Windows) support for hardware-based key protection.
  - Keys are sealed to the hardware and are not exposed in plaintext.
  - Deterministic Attribution: Agent IDs (`aid:...`) are derived from hardware identity.
- **ZK-STARK Audit Trails**:
  - High-performance proofs using Goldilocks fields and Two-Adic FRI.
  - **Audit-as-Code**: Generate proofs verifiable by third parties without data exposure.
- **Quantum Undo System**:
  - **Reversible State**: Automatic filesystem snapshots before every `attest exec`.
  - **Instant Rollback**: Revert to known-verified states via `attest quantum undo`.
- **Pure-Go SQLite Bridge**: 100% portable CGO-free storage for seamless deployment anywhere.


## [v0.1.0-alpha] - 2026-02-05

### Added
- **ZK-STARK Integrity**: Integrated **Plonky3** framework with custom `AuditAir` constraint system.
- **Hardware-Backed Identity**: Native TPM2 (Linux) and CNG (Windows) support for sealed identities.

---

## [v0.0.1] - 2025-02-01

### Added
- **Core Strategy**: Implementation of Agent Identity (`aid:<hash>`) and Ed25519 signing.
- **Policy Engine**: Rule-based control for dangerous/destructive command execution.
- **Git Integration**: Pre-commit hooks and automated attestation for git workflows.
- **Multi-SDK Support**: Initial release of Python and JavaScript SDKs.
- **Storage**: Initial SQLite-based persistence layer.

---

## [v0.0.0] - 2024-12-01

- Initial development release and architectural proof-of-concept.
