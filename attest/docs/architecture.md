# Architecture: Verifiable Reasoning and Provenance

The **Attest Protocol** provides a cryptographic foundation for autonomous agent ecosystems like **VEX**. It offers specific guarantees regarding identity and state:

1.  **Hardware-Rooted Identity**: Provides evidence that an action originated from a specific physical chip.
2.  **ZK-STARK Verification**: Provides evidence that an agent's cognitive state transitioned according to defined rules, without exposing sensitive variables.

When integrated together, `attest`, `vex-hardware`, and the `vex` execution runtimes create a verifiable "Glassbox" architecture.

---

## The Protocol Anatomy

```mermaid
graph TD
    %% VEX Cognitive Layer
    subgraph Cognitive Layer (VEX Protocol)
        Agent([VEX Agent / LLM])
        Orchestrator[Agent Orchestrator]
        Exec[Agent Executor]
        Mem[Temporal Memory]
        
        Agent --> Orchestrator
        Orchestrator --> Exec
        Orchestrator --> Mem
    end

    %% Hardware Identity Layer
    subgraph Hardware Identity (vex-hardware)
        TEE[Trusted Execution Environment TPM2/CNG]
        Signer[Ed25519 Hardware Signer]
        Zeroize[Memory Zeroization Dropper]
        
        TEE --> Signer
        Signer --> Zeroize
    end

    %% Audit & Proof Layer
    subgraph Verification Layer (Attest-RS)
        Store[(vex-persist AuditStore)]
        Merkle[Merkle Hash Chain]
        Plonky3[Plonky3 ZK-STARK Prover]
        
        Store --> Merkle
        Merkle --> Plonky3
    end

    %% Integration Paths
    Exec -- "1. Request Signature" --> Signer
    Signer -- "2. Signed Execution Event" --> Store
    
    Orchestrator -- "3. Evolve Genome" --> Signer
    Signer -- "4. Signed Mutation Event" --> Store
    
    Plonky3 -- "5. Succinct Verification" --> User([Human Auditor])
```

---

## 🏗️ Layer 1: Hardware Identity (`vex-hardware`)

True agent accountability begins at the silicon level. Software-only private keys can be stolen by malware or inadvertently committed to GitHub.

`vex-hardware` solves this by forcing the physical machine to act as the agent's identity broker.

*   **Linux (TPM2)**: Interfaces via `tss-esapi` to generate primary non-exportable seeds sealed inside the Trusted Platform Module.
*   **Windows (CNG)**: Interfaces via `windows-sys` and Cryptography Next Generation to utilize the Microsoft Platform Crypto Provider.
*   **Mandatory Zeroization**: To provide blazing-fast signature throughput for multi-agent swarms, the Ed25519 signing seeds are briefly held in RAM. However, `vex-hardware` implements the strict `Zeroize` trait. The millisecond the signing operation finishes and the variable goes out of scope, the memory address is forcefully overwritten with zeroes, preventing cold-boot or memory-scraping attacks.

---

## 🧠 Layer 2: Cognitive Binding (VEX Runtime)

The VEX Protocol is the "brain" (the LLM routing, the debate mechanisms, the temporal memory). Attest is the "nervous system."

In the `vex-runtime` crate, the `AgentExecutor` and the `Orchestrator` are initialized with an `AgentIdentity` (from `vex-hardware`). 

### The Lifecycle of a Signed Thought
1.  **Execution**: An agent decides to take an action (e.g., query a database or execute a tool).
2.  **Serialization**: The VEX runtime deterministically serializes the exact parameters of that thought (using JCS - RFC 8785).
3.  **Hardware Signing**: The `vex-hardware` signer is invoked, producing an Ed25519 signature over that state.
4.  **Genome Anchoring**: If the `Orchestrator` determines that an agent needs to evolve (e.g., modifying its system prompt based on poor performance), a `GenomeEvolved` event is generated. This mutation is also hardware-signed, generating an immutable record of *why* the AI changed its own mind.

---

## 🛡️ Layer 3: Mathematical Verification (`attest-rs`)

Every signed action and genome mutation is stored sequentially in `vex-persist`'s `AuditStore`. To prevent tampering, these events are hashed into a continuous Merkle chain.

However, auditing a chain of a million AI thoughts is computationally expensive. **This is where `attest-rs` and ZK-STARKs come in.**

*   **Plonky3 Integration**: `attest-rs` utilizes the Plonky3 framework to generate FRI (Fast Reed-Solomon Interactive Oracle Proofs of Proximity).
*   **AuditAir**: We define a custom Arithmetic Intermediate Representation (`AuditAir`). This is a polynomial constraint system that mathematically dictates: *"An audit log is only valid if Hash C properly incorporated Hash B, which properly incorporated Hash A."*
*   **Recursive Zero-Knowledge**: The prover compresses the entire history of the agent's actions into a single succinct proof. A human auditor (or a smart contract) can verify millions of complex state transitions in milliseconds, mathematically guaranteeing the agent followed the rules without ever needing to read the raw log.

### Security Hardening
The STARK circuitry in `attest-rs` is rigorously hardcoded against:
*   **Serialization Corruption**: The prover will mathematically reject any trace where the inputs were tampered with mid-flight.
*   **Public Input Forgery**: The proof is cryptographically bound to the final state of the Merkle root, ensuring a malicious actor cannot generate a valid proof for a fake timeline.
