# Verifiable Agent Receipt (.capsule)
## VEX × CHORA Joint Specification v0.2 (PCR Binding)

**Authors:** Quinten Stroobants (VEX), George Lagogiannis (CHORA)

A `.capsule` is a portable, cryptographically-sealed artifact that proves an AI agent's action was **intended, authorized, and hardware-rooted**. It provides a definitive, tamper-proof record of autonomous execution, verifiable offline by any third party without access to the internal logic or state of the originating nodes.

---

## 1. Commitment Surface

The commitment surface is built upon **JCS (JSON Canonicalization Scheme - RFC 8785)** and **SHA-256**. This combination ensures that the commitment is deterministic across different platforms, languages, and JSON implementations.

### Pillar Hashing (Canonical Scopes)
To ensure mathematical parity across nodes (VEX, CHORA, ATTEST), only the following fields are included in the JCS hashing scope for each pillar. Any extra metadata fields in the JSON MUST be excluded from the hash.

| Pillar | Canonical Hashing Scope (Surface Type) | Fields Included |
|--------|----------------------------------------|-----------------|
| **Intent** | **Inclusive** (Models + Metadata) | `request_sha256`, `confidence`, `capabilities`, `magpie_source`, `*` (All extra fields) |
| **Authority** | **Inclusive** (Models + Metadata) | `capsule_id`, `outcome`, `reason_code`, `trace_root`, `nonce`, `gate_sensors`, `*` (All extra fields) |
| **Identity** | **Inclusive** (Models + Metadata) | `aid`, `identity_type`, `pcrs`, `*` (All extra fields) |
| **Witness** | **Minimal** (Explicit Fields Only) | `chora_node_id`, `receipt_hash`, `timestamp` |

### Capsule Root (The Commitment)
The `capsule_root` is the single canonical commitment for the entire artifact. When building the root object for JCS, field keys **MUST** be ordered lexicographically.

```
capsule_root = SHA256(JCS({
  "authority_hash": authority_hash,
  "identity_hash":  identity_hash,
  "intent_hash":    intent_hash,
  "witness_hash":   witness_hash
}))
```

> [!NOTE]
> The `request_commitment` field (v0.2 Additive) is explicitly **EXCLUDED** from the `capsule_root` calculation to maintain backward compatibility with v0.1 signatures.

**Signature Surface:** The 32-byte binary `capsule_root` is the direct input to the signature algorithm: `Ed25519(private_key, capsule_root)`.

---

## 2. Segments (Data Structures)

### Intent (VEX Pillar)
Documents the agent's internal state and formal reasoning prior to execution.
```json
{
  "request_sha256": "hex[32] (Payload commitment)",
  "confidence": "float64 (0.0 - 1.0)",
  "capabilities": ["string (e.g., 'filesystem', 'network')"],
  "magpie_source": "string (Optional - bundled UTF-8 formal AST)",
  "...": "Any (Flattened Extra Metadata - Included in Hash)"
}
```

### Authority (CHORA Pillar)
Documents the governance decision and the cryptographic trace allowed by the gatekeepers.
```json
{
  "capsule_id": "string (UUID v4)",
  "outcome": "ALLOW | HALT | ESCALATE",
  "reason_code": "string (e.g., 'POLICY_MATCH')",
  "trace_root": "hex[32] (Cryptographic policy trace)",
  "nonce": "uint64 (Strictly increasing counter)",
  "gate_sensors": "object (Optional gate sensor telemetry)",
  "...": "Any (Flattened Extra Metadata - Included in Hash)"
}
```

### Identity (Attest Pillar)
Proves the hardware source (Silicon) and its boot/runtime integrity state.
```json
{
  "aid": "string (Attest ID - 32-byte hex)",
  "identity_type": "string (e.g., 'tpm2.0', 'cng')",
  "pcrs": {
    "0": "hex[32] (SRTM - System Measurement)",
    "7": "hex[32] (Secure Boot Policy)",
    "11": "hex[32] (Kernel Runtime Integrity)"
  },
  "...": "Any (Flattened Extra Metadata - Included in Hash)"
}
```
*Constraints:* PCR indices **MUST** be represented as strings in the JSON map to satisfy JCS deterministic sorting requirements.

### Witness (Log Pillar)
The third-party custody record from the witness network. This pillar uses a **minimal hashing scope** to ensure cross-stack interoperability.
```json
{
  "chora_node_id": "string (Authority Node ID)",
  "receipt_hash": "hex (Authority signature of the root)",
  "timestamp": "uint64 (Unix Epoch - Seconds)"
}
```
*Note: Metadata fields like `witness_mode` or `sentinel_mode` may be present in the JSON but ARE NOT hashed.*

---

## 3. Verification Segment (The Root Envelope)
The JSON representation of the capsule, typically used for API transmission and storage.

```json
 {
   "capsule_id": "string (UUID)",
   "intent": "IntentSegment",
   "authority": "AuthoritySegment",
   "identity": "IdentitySegment",
   "witness": "WitnessSegment",
   "intent_hash": "hex",
   "authority_hash": "hex",
   "identity_hash": "hex",
   "witness_hash": "hex",
   "capsule_root": "hex",
   "crypto": {
     "algo": "ed25519",
     "public_key_endpoint": "url (Key discovery)",
     "signature_scope": "capsule_root",
     "signature_b64": "base64 (The hardware seal)"
   },
   "request_commitment": {
    "canonicalization": "JCS-RFC8785",
    "payload_sha256": "hex[32]",
    "payload_encoding": "application/json"
   }
 }
```

---

## 4. Wire Format (VEP Binary Envelope)

Low-latency/High-performance binary format for edge devices and audit log streaming.

### Header (76 bytes)
The header contains fixed-width fields for zero-copy parsing.
```
magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
```
- **magic**: `0x564550` ("VEP")
- **version**: `0x03` (VEX v1.3.0 / PCR-Bound)
- **aid**: 32-byte Hardware Identity Hash
- **capsule_root**: 32-byte Binary Commitment Root
- **nonce**: 8-byte Replay Protection Counter (Big-Endian)

### Binary Body (TLV Segments)
Following the header, the body consists of Type-Length-Value segments.

| Type | Name | Content |
|---|---|---|
| 0x01 | Intent | JCS JSON of IntentSegment |
| 0x02 | Authority | JCS JSON of AuthoritySegment |
| 0x03 | Identity | JCS JSON of IdentitySegment |
| 0x05 | Witness | JCS JSON of WitnessSegment |
| 0x06 | Signature | Raw 64-byte Ed25519 signature |
| 0x07 | MagpieAst | Raw UTF-8 Magpie formal source code |

---

## 5. Offline Verification Flow (Definitive)

1.  **Header Deconstruction**: Extract binary `capsule_root`, `aid`, and `nonce`.
2.  **TLV Segment Extraction**: Traverse the binary body. Ensure all 4 primary pillars (Intent, Authority, Identity, Witness) and the Signature are present.
3.  **Cross-Field Integrity**:
    - Verify `identity.aid` matches header `aid`.
    - Verify `authority.nonce` matches header `nonce`.
4.  **Silicon State Audit**: Compare `identity.pcrs` against known-good "Golden PCR" states for the deployment environment.
5.  **Recompute Commitment Path**:
    - Recompute pillar hashes: `SHA256(JCS(Segment))`.
    - Recompute `capsule_root` using JCS lexicographical ordering of the four hashes.
6.  **Cryptographic Validation**: Verify the binary `Signature` (Type 6) against the recomputed `capsule_root`.
7.  **Formal Intent Check (Optional)**: Execute `magpie parse` on the `MagpieAst` (Type 7) to confirm the formal logic matches the authorized `trace_root`.

**Reference Parity Vector (v0.2):**
Implementation-specific test vectors for v0.2 are provided in the `vex-runtime` test suite. The `capsule_root` calculation MUST remain stable across all compliant implementations.

---

## 6. Design Principles & Constraints

- **Zero Float Ambiguity**: All internal hashing logic MUST use integers or strings.
- **Lexicographical Integrity**: Objects MUST be sorted by key during JCS canonicalization.
- **Structural Hardening**: Intent, Authority, and Identity pillars use an **Inclusive** hashing surface. Any field present in the JSON at these levels (flattened metadata) MUST be captured and included in the JCS hash to ensure binary parity for extended protocols.
- **Minimal Witness Compliance**: The Witness pillar uses an **Explicit** hashing surface. Only the three defined fields are hashed; all other witness fields (e.g., `witness_mode`) sit outside the cryptographic commitment.
- **Omission over Null**: Optional fields that are empty MUST be omitted from the JSON rather than set to `null` to minimize artifact size.
- **Hardware-Rooted Seal**: The signature MUST be the FINAL operation, sealing the hardware identity and PCR state into the witness receipt.
- **OTS Finality**: External anchoring (e.g., OpenTimestamps) is performed *after* the capsule is sealed and witnessed.
