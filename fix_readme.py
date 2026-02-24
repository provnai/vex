
import sys

with open('README.md', 'r', encoding='utf-8') as f:
    lines = f.readlines()

new_bullets = [
    '- 🛡️ **Blue Agent Reflection** - Agents now reconsider their stances based on debate arguments, eliminating hardcoded bias.\n',
    '- ⚡ **O(1) API Key Verification** - Instant auth lookups using UUID prefixes to prevent DoS attacks.\n',
    '- 🔒 **Isolated Multi-Tenancy** - Strictly bounded context, storage, and rate-limiting per-tenant.\n',
    '- 🧊 **Fortified Replay Protection** - TTL-based nonce caching with `moka` and mandatory capacity bounds.\n',
    '- 🚀 **Worker Robustness** - Graceful handling of malformed job payloads without panicking worker threads.\n'
]

lines[46:51] = new_bullets

with open('README.md', 'w', encoding='utf-8') as f:
    f.writelines(lines)
