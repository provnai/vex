# .capsule — Verifiable Agent Receipt
## VEX × CHORA Joint Specification v0.1 (LOCKED)

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
  "id": "string",
  "goal": "string",
  "description": "string (optional)",
  "ticketId": "string (optional)",
  "constraints": [],
  "acceptanceCriteria": [],
  "status": "string",
  "createdAt": "RFC3339 UTC",
  "closedAt": "RFC3339 UTC (optional)"
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
  "agent": "string",
  "tpm": "string"
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

## 3. Wire Format (VEP Header — 76 bytes)

```
magic(3) | version(1) | aid(32) | capsule_root(32) | nonce(8)
```

- `magic`: `0x564550` ("VEP")
- `version`: `0x02`
- `aid`: Attest ID — hardware-rooted identity hash
- `capsule_root`: The canonical commitment root
- `nonce`: 8-byte replay protection counter

---

## 4. Verification Flow (Offline)

1. **Parse** the JCS structure and extract the four segments.
2. **Recompute** the four pillar hashes from raw segment bytes.
3. **Recompute** `capsule_root` (ensuring JCS lexicographical sorting).
4. **Verify** the Ed25519 signature against the CHORA public key.

**Reference parity vector (Consensus v0.1):**
- **Intent Hash**: `db3bcbbe6796d6ae763e752306941b0159fff0b86043be21889e8db1ecf42baa`
- **Authority Hash**: `b4865793e475cb3170d8f6574ac82ab3068af648cf2f120bc616c0c7e8fd403a`
- **Identity Hash**: `367747b4379df5fb142a9672c0d8663eed95fb6fbcbfb99555bb58c6714f3e93`
- **Witness Hash**: `de78acc160b76505cab011011ff8c50878ed14b37da6a39131f185ec29291c32`

**Definitive Capsule Root:**
`c07b0a4e9c49c861d69205a82fef35379894771ac30927b3d1ac48d5b36d9d71`

---

## 5. Design Principles

- **No floats.** All values are integers or strings to avoid drift.
- **None fields are omitted.** Do not use `null` keys.
- **Witness before Silicon.** The log record is appended prior to the final seal.
- **OTS Finality is post-seal.** The Bitcoin timestamp covers the completed capsule.
