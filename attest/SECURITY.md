# Security Documentation

This document describes the security model, practices, and considerations for Attest v0.1.0.

## Threat Model

Attest is designed to provide cryptographic verification of AI agent actions. The following threats are in scope:

### Threats Addressed

1. **Agent Impersonation**: Attest prevents unauthorized agents from acting under someone else's identity by using Ed25519 key pairs tied to agent IDs.

2. **Action Tampering**: Once an action is attested, it cannot be modified without invalidating the cryptographic signature.

3. **Intent Misrepresentation**: The intent system ensures agents must declare their goals before acting, with cryptographic linkage between intent and execution.

4. **Audit Trail Gaps**: All attestations are stored in an immutable SQLite database with tamper-evident properties.

5. **Dangerous Command Execution**: The policy engine can block dangerous commands before they execute.

### Threats Out of Scope

1. **Key Extraction**: If an attacker gains access to the private key, they can forge attestations. Key storage security is the user's responsibility.

2. **Side Channels**: Timing attacks on key operations are possible but require local access.

3. **Social Engineering**: Attest cannot prevent an agent from being manipulated into creating malicious intents.

4. **Physical Security**: Physical access to the machine can bypass software protections.

## Security Practices

### Key Storage & Hardware Identity

Attest uses Ed25519 keys for all cryptographic operations. Keys are stored with hardware backing where available:

- **Linux**: Native TPM2 support via `tss-esapi`. Keys are sealed to the TPM device and never touch the filesystem in plaintext.
- **Windows**: Microsoft Platform Crypto Provider (CNG). Keys are protected by the system's TPM.
- **Legacy/Other**: Stored in `~/.attest/keys/<agent-id>/private.key` with 0600 permissions.

**Recommendations:**
- Always use TPM-backed identities on supported hardware.
- Use filesystem encryption (e.g., LUKS, BitLocker) for fallback storage.
- Never commit keys to version control.
- Rotate keys periodically: `attest agent create --rotate-from <old-id>`

### Signing

All attestations use Ed25519 signatures:

```
Signature = Ed25519Sign(private_key, attestation_data)
```

Attestation data includes:
- Agent ID
- Intent ID (if linked)
- Command/executed action
- Timestamp
- Environment metadata
- Previous attestation hash (chain)

### Verification

Verification is performed by:
1. Extracting the public key from the agent record
2. Verifying the Ed25519 signature against attestation data
3. Checking the agent is not revoked
4. Validating the attestation chain (if applicable)

## Known Limitations

### Cross-Platform CGO-Free Storage

Attest uses `modernc.org/sqlite` (Pure Go) for SQLite storage on the Go bridge and `sqlx` with `sqlite` features in Rust. 

1. **Safety**: No external C dependencies eliminates memory safety issues typical of C bridge drivers.
2. **Portability**: The system builds and runs exactly as expected on Windows, Linux, and Darwin without complex toolchains.
3. **Static Linking**: The Rust core is statically linked into the Go bridge for absolute deployment simplicity.

### Key Access on Shared Systems

On shared systems, other users may be able to:
- Read agent keys if permissions are incorrect (mitigated by 0600)
- Monitor memory for key material during signing (requires local access)

### No HSM Integration (v0.1.0)

v0.1.0 does not support hardware security modules beyond TPM/CNG.

**Planned for v0.3.0:**
- PKCS#11 HSM support
- AWS KMS integration
- Azure Key Vault integration

## Reporting Vulnerabilities

### Responsible Disclosure

We follow responsible disclosure practices for security vulnerabilities:

1. **Do NOT** report vulnerabilities in public issues
2. **Do NOT** attempt to exploit vulnerabilities on production systems
3. **Do NOT** share vulnerability details with third parties

### Reporting Process

1. Email: security@attest-project.example.com
2. Include:
   - Description of the vulnerability
   - Steps to reproduce
   - Potential impact
   - Suggested mitigation (if any)
3. You will receive acknowledgment within 24 hours
4. We aim to provide a fix within 90 days

### What to Expect

- Initial response within 24 hours
- Status updates every 7 days
- Credit in release notes (unless you request anonymity)
- Security advisory published with the fix

## Cryptography Details

### Ed25519

Attest uses Ed25519 for all digital signatures:

- **Algorithm**: Ed25519 (RFC 8032)
- **Key Size**: 256-bit (32-byte public, 64-byte private)
- **Signature Size**: 64 bytes
- **Security Level**: 128-bit security (equivalent to ~3072-bit RSA)

#### Key Generation

```go
// From pkg/crypto/keys.go
func GenerateEd25519KeyPair() (*KeyPair, error) {
    privateKey := make([]byte, ed25519.PrivateKeySize)
    _, err := io.ReadFull(rand.Reader, privateKey)
    if err != nil {
        return nil, err
    }
    // ... key derivation
}
```

Keys are generated using `crypto/rand` for cryptographic randomness.

#### Signing Process

```go
signature := ed25519.Sign(privateKey, data)
```

#### Verification Process

```go
valid := ed25519.Verify(publicKey, data, signature)
```

### ZK-STARK Audit Integrity (Plonky3)

Since the Alpha release, Attest uses Plonky3 to generate succinct proofs of audit trail integrity. This ensures that the audit log hasn't been tampered with, even if the storage layer is compromised.

#### AuditAir Constraints
Our ZK-STARK implementation (`AuditAir`) enforces that:
- Every state transition follows the rule: `next_state = current_state + event_influence`.
- The `event_influence` is cryptographically derived from the `event_hash`.
- The starting state matches the previous audit root.

#### Proving Flow
1. **Trace Generation**: The auditor generates an execution trace of state transitions.
2. **STARK Generation**: Using the Goldilocks field, a succinct proof is generated using the Plonky3 prover.
3. **Commitment**: A STARK commitment is attached to the audit entry.

#### Verification
Verification is fast and constant-time relative to the log size:
```bash
attest verify --zk <attestation-id>
```

### Agent ID Derivation

Agent IDs are derived from the public key:

```
aid:<first-8-bytes-of-sha256(public-key)>
```

This provides:
- Uniqueness: 64-bit hash space
- Verifiability: Anyone can derive the ID from the public key
- Compactness: 20-character identifier

### Hash Functions

Attest uses SHA-256 for:
- Agent ID derivation
- Checksum generation for release artifacts
- Chain linking between attestations

### Random Number Generation

- Key generation: `crypto/rand` (OS-provided CSPRNG)
- Nonces: Random 32-byte values for each attestation
- UUIDs: Generated using OS entropy

## Data Privacy

### Collected Data

Attest may collect:
- Agent metadata (name, type, framework, model)
- Intent descriptions and goals
- Command executed
- Environment variables (if configured)
- File paths modified

### Data Storage

- All data stored locally in `~/.attest/`
- No data transmitted to external servers
- User controls all data retention

### Recommendations

1. Use generic intent descriptions when possible
2. Avoid including sensitive data in intent goals
3. Use environment filtering to exclude secrets
4. Implement data retention policies

## Compliance Considerations

### Audit Readiness

Attest is designed for audit compliance:
- Immutable attestation records
- Cryptographic proof of actions
- Full chain of custody from intent to execution

### Data Retention

Organizations should:
- Implement backup and archival policies
- Consider encryption for archived attestations
- Define retention periods based on compliance requirements

### Access Control

- File permissions (0600 for keys, 0700 for directories)
- User isolation on multi-user systems
- Integration with existing IAM systems

## Security Best Practices

### For Individual Users

1. Keep your attest binary updated
2. Use filesystem encryption
3. Rotate keys periodically
4. Review attestations before committing

### For Organizations

1. Centralized key management (v0.3.0)
2. Integration with existing HSMs (v0.3.0)
3. Audit log aggregation
4. Policy governance
5. Regular security reviews

### For CI/CD

1. Use dedicated service accounts
2. Store keys securely (Vault, Secrets Manager)
3. Rotate CI/CD keys frequently
4. Monitor attestation patterns

## References

- [RFC 8032 - Ed25519](https://datatracker.ietf.org/doc/html/rfc8032)
- [SQLite Security Considerations](https://www.sqlite.org/security.html)
- [Cobra Security Best Practices](https://github.com/spf13/cobra/blob/main/docs/security.md)
- [Go Crypto Guidelines](https://golang.org/security/crypto/)

## Version

This document applies to Attest v0.1.0.

Last updated: 2026-03-05
