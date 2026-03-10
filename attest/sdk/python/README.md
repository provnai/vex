# Attest Agent Python SDK

A Python SDK for the [Attest](https://github.com/anomalyco/attest) verifiable agent action system.

## Features

- **Agent Management**: Create, list, and manage cryptographic agent identities
- **Attestation**: Sign and verify agent actions with cryptographic attestations
- **Intent Tracking**: Track the goals and intentions behind agent actions
- **Reversible Execution**: Execute commands with automatic backup and rollback support
- **Policy Enforcement**: Check actions against safety policies
- **LangChain Integration**: Automatic attestation recording for LangChain agents

## Installation

```bash
pip install attest-agent
```

For LangChain integration:

```bash
pip install attest-agent[langchain]
```

## Quick Start

```python
from attest import AttestClient

client = AttestClient()

# Create an agent
agent = client.agent_create(
    name="my-assistant",
    agent_type="langchain"
)

# Create an attestation
attestation = client.attest_action(
    agent_id=agent.id,
    action="command",
    target="python script.py"
)

# Verify the attestation
result = client.verify_attestation(attestation.id)
print(f"Valid: {result['valid']}")
```

## Documentation

See [](docs/python-sdkdocs/python-sdk.md.md) for complete documentation including:

- API reference
- LangChain integration guide
- Full workflow examples
- Data model documentation

## License

MIT
