# Attest JavaScript SDK

A JavaScript/Node.js SDK for the [Attest](https://github.com/anomalyco/attest) verifiable agent action system.

## Features

- **Agent Management**: Create, list, and manage cryptographic agent identities
- **Attestation**: Sign and verify agent actions with cryptographic attestations
- **Intent Tracking**: Track the goals and intentions behind agent actions
- **Reversible Execution**: Execute commands with automatic backup and rollback support
- **Policy Enforcement**: Check actions against safety policies
- **CLI Tool**: Convenient command-line interface for quick operations

## Installation

```bash
npm install @attest/sdk
```

## Quick Start

```javascript
const { AttestClient } = require('@attest/sdk');

const client = new AttestClient();

async function main() {
  const agent = await client.agentCreate('my-assistant', {
    type: 'langchain',
    metadata: { model: 'gpt-4' }
  });
  console.log('Created agent:', agent.id);

  const attestation = await client.attestAction(
    agent.id,
    'command',
    'python script.py',
    { intentId: 'int:task-123' }
  );
  console.log('Created attestation:', attestation.id);

  const verification = await client.verifyAttestation(attestation.id);
  console.log('Verified:', verification.valid);
}

main().catch(console.error);
```

## Documentation

See [docs/js-sdk.md](docs/js-sdk.md) for complete documentation including:

- API reference
- Full workflow examples
- TypeScript usage
- Error handling
- Configuration options

## CLI Tool

The SDK includes a CLI tool for quick operations:

```bash
# List agents
attest-agent agent list

# Create an agent
attest-agent agent create my-agent --type langchain

# Show agent details
attest-agent agent show aid:12345678

# List intents
attest-agent intent list

# Verify an attestation
attest-agent verify att:abcdef
```

## Requirements

- Node.js 16.0.0 or higher
- Attest CLI (>=0.1.0)

## License

MIT License - see the Attest repository for details.
