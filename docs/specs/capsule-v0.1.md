# .capsule — Verifiable Agent Receipt
## VEX × CHORA Joint Specification v0.1 (LOCKED - Hardened)

A `.capsule` is a portable, cryptographically-sealed artifact that proves an AI agent's action was **intended, authorized, and hardware-rooted** — verifiable offline by any third party without access to either node's internal logic.

---

## 1. Commitment Surface

Each pillar is hashed independently using JCS canonicalization (RFC 8785) + SHA-256:

```
intent_hash    = SHA256(JCS(intent))
authority_hash = SHA256(JCS(authority))
identity_hash  = SHA256(JCS(identity))
witness_hash   = SHA256(JCS(witness))

capsule_root = SHA256(JCS({
  "intent_hash":    intent_hash,
  "authority_hash": authority_hash,
  "identity_hash":  identity_hash,
  "witness_hash":   witness_hash
}))
```

The `capsule_root` is the single canonical commitment. Field keys **MUST** be ordered lexicographically.

**Signature surface:** `Ed25519(private_key, capsule_root)`

---

## 2. Segments

### Intent (VEX)
Proves the proposed action before execution.
```json
{
  "request_sha256": "hex[32]",
  "confidence": "float64 (0.0 - 1.0)",
  "capabilities": ["string"]
}
```

### Authority (CHORA)
Proves the governance decision. 
```json
{
  "capsule_id": "string",
  "outcome": "ALLOW | HALT | ESCALATE",
  "reason_code": "string",
  "trace_root": "hex[32]",
  "nonce": "uint64 execution nonce"
}
```

### Identity (Attest)
Proves the hardware source (Silicon).
```json
{
  "aid": "string (Attest ID)",
  "identity_type": "string"
}
```

### Witness (Log)
Proves the custody record was independently appended.
```json
{
  "chora_node_id": "string",
  "receipt_hash": "hex",
  "timestamp": "RFC3339 UTC"
}
```

---

## 3. Verification Segment (Root)
Binds the pillars into a single commitment.
```json
 {
   "capsule_id": "string",
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
     "public_key_endpoint": "string",
     "signature_scope": "capsule_root",
     "signature_b64": "base64"
   }
 }
```

---

## 4. Wire Format (VEP Header — 76 bytes)

```
magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
```

- `magic`: `0x564550` ("VEP")
- `version`: `0x02`
- `aid`: Attest ID — hardware-rooted identity hash
- `capsule_root`: The canonical commitment root
- `nonce`: 8-byte replay protection counter

---

## 5. Verification Flow (Offline)

1. **Parse** the JCS structure and extract the four segments.
2. **Recompute** the four pillar hashes from raw segment bytes.
3. **Recompute** `capsule_root` (ensuring JCS lexicographical sorting).
4. **Verify** the Ed25519 signature against the CHORA public key.

**Reference parity vector (Consensus v0.1):**
- **Intent Hash**: `e02504ea88bd9f05a744cd8a462a114dc2045eb7210ea8c6f5ff2679663c92cb`
- **Authority Hash**: `6fac0de31355fc1dfe36eee1e0c226f7cc36dd58eaad0aca0c2d3873b4784d35`
- **Identity Hash**: `7869bae0249b33e09b881a0b44faba6ee3f4bab7edcc2aa5a5e9290e2563c828`
- **Witness Hash**: `174dfb80917cca8a8d4760b82656e78df0778cb3aadd60b51cd018b3313d5733`

**Definitive Capsule Root:**
`71d0324716f378b724e6186340289ecad5b99d6301d1585a322f2518db52693e`

---

## 6. Design Principles

- **No floats in hashing.** All values are integers or strings during JCS to avoid drift.
- **None fields are omitted.** Do not use `null` keys.
- **Witness before Silicon.** The log record is appended prior to the final seal.
- **OTS Finality is post-seal.** The Bitcoin timestamp covers the completed capsule.
