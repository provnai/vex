"""Attest Python SDK.

A Python SDK for the Attest verifiable agent action system.

Modules:
    attest_client: Main client for interacting with the Attest CLI.
    langchain_callback: LangChain callback handler for automatic attestation.

Quick Start:
    >>> from attest import AttestClient
    >>> client = AttestClient()
    >>> agent = client.agent_create(name="my-agent", agent_type="langchain")
"""

from attest_client import (
    AttestClient,
    AttestError,
    AttestCLIError,
    AttestConfigurationError,
    AttestNotFoundError,
    Agent,
    Attestation,
    Intent,
    ExecutionResult,
)

from langchain_callback import (
    AttestCallbackHandler,
    create_attest_callback,
    create_managed_callback,
)

__all__ = [
    "AttestClient",
    "AttestError",
    "AttestCLIError",
    "AttestConfigurationError",
    "AttestNotFoundError",
    "Agent",
    "Attestation",
    "Intent",
    "ExecutionResult",
    "AttestCallbackHandler",
    "create_attest_callback",
    "create_managed_callback",
]

__version__ = "0.1.0"
