"""Python client for the Attest verifiable agent action system.

This module provides a Python SDK for interacting with the Attest CLI,
enabling programmatic creation and verification of agent attestations.

Example:
    >>> from attest_client import AttestClient
    >>> client = AttestClient()
    >>> agent = client.agent_create(name="my-agent", agent_type="langchain")
    >>> client.attest_action(agent_id=agent["id"], action="file_edit", target="test.py")
"""

import json
import os
import subprocess
import sys
from dataclasses import dataclass, field
from typing import Any, Dict, List, Optional, Union
from pathlib import Path


class AttestError(Exception):
    """Base exception for Attest SDK errors."""

    def __init__(
        self,
        message: str,
        return_code: Optional[int] = None,
        stderr: Optional[str] = None,
    ):
        self.message = message
        self.return_code = return_code
        self.stderr = stderr
        super().__init__(self._format_message())

    def _format_message(self) -> str:
        parts = [self.message]
        if self.return_code is not None:
            parts.append(f"(return code: {self.return_code})")
        if self.stderr:
            parts.append(f"stderr: {self.stderr}")
        return " ".join(parts)


class AttestCLIError(AttestError):
    """Exception raised when the Attest CLI returns an error."""

    pass


class AttestConfigurationError(AttestError):
    """Exception raised when Attest is not properly configured."""

    pass


class AttestNotFoundError(AttestError):
    """Exception raised when an entity (agent, attestation, etc.) is not found."""

    pass


@dataclass
class Agent:
    """Represents an agent identity in the Attest system.

    Attributes:
        id: Unique identifier for the agent (e.g., "aid:12345678").
        name: Human-readable name for the agent.
        type: Type of agent (e.g., "generic", "langchain", "autogen").
        public_key: Agent's public key for signature verification.
        created_at: ISO 8601 timestamp of creation.
        revoked: Whether the agent has been revoked.
        revoked_at: ISO 8601 timestamp of revocation, if applicable.
        metadata: Additional metadata as a dictionary.
    """

    id: str
    name: str
    type: str
    public_key: str
    created_at: str
    revoked: bool = False
    revoked_at: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Agent":
        """Create an Agent from a dictionary."""
        return cls(
            id=data.get("id", ""),
            name=data.get("name", ""),
            type=data.get("type", ""),
            public_key=data.get("publicKey", ""),
            created_at=data.get("createdAt", ""),
            revoked=data.get("revoked", False),
            revoked_at=data.get("revokedAt"),
            metadata=data.get("metadata", {}),
        )


@dataclass
class Attestation:
    """Represents a cryptographic attestation of an agent action.

    Attributes:
        id: Unique identifier for the attestation (e.g., "att:12345678").
        agent_id: ID of the agent that created this attestation.
        agent_name: Name of the agent.
        intent_id: Optional ID of the associated intent.
        action_type: Type of action attested (e.g., "command", "file_edit").
        action_target: Target of the action (e.g., command string, file path).
        action_input: Input provided to the action.
        timestamp: ISO 8601 timestamp of the attestation.
        signature: Cryptographic signature of the attestation.
        metadata: Additional metadata as a dictionary.
    """

    id: str
    agent_id: str
    agent_name: str
    action_type: str
    action_target: str
    timestamp: str
    signature: str
    intent_id: Optional[str] = None
    action_input: Optional[str] = None
    metadata: Dict[str, Any] = field(default_factory=dict)

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Attestation":
        """Create an Attestation from a dictionary."""
        return cls(
            id=data.get("id", ""),
            agent_id=data.get("agentId", ""),
            agent_name=data.get("agentName", ""),
            intent_id=data.get("intentId"),
            action_type=data.get("actionType", ""),
            action_target=data.get("actionTarget", ""),
            action_input=data.get("actionInput"),
            timestamp=data.get("timestamp", ""),
            signature=data.get("signature", ""),
            metadata=data.get("metadata", {}),
        )


@dataclass
class Intent:
    """Represents an intent record for agent actions.

    Attributes:
        id: Unique identifier for the intent.
        goal: The goal or objective of the intended action.
        constraints: Constraints on how the goal should be achieved.
        acceptance_criteria: Criteria for accepting completion.
        status: Current status of the intent.
        agent_id: ID of the associated agent.
        created_at: ISO 8601 timestamp of creation.
        completed_at: ISO 8601 timestamp of completion, if applicable.
    """

    id: str
    goal: str
    status: str = "pending"
    constraints: Dict[str, Any] = field(default_factory=dict)
    acceptance_criteria: List[str] = field(default_factory=list)
    agent_id: Optional[str] = None
    created_at: Optional[str] = None
    completed_at: Optional[str] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "Intent":
        """Create an Intent from a dictionary."""
        return cls(
            id=data.get("id", ""),
            goal=data.get("goal", ""),
            status=data.get("status", "pending"),
            constraints=data.get("constraints", {}),
            acceptance_criteria=data.get("acceptanceCriteria", []),
            agent_id=data.get("agentId"),
            created_at=data.get("createdAt"),
            completed_at=data.get("completedAt"),
        )


@dataclass
class ExecutionResult:
    """Result of a reversible execution.

    Attributes:
        id: Unique identifier for the execution.
        command: The command that was executed.
        working_dir: Working directory for the execution.
        backup_path: Path to the backup, if created.
        status: Status of the execution (executed, failed, rolled_back).
        created_at: ISO 8601 timestamp of creation.
        rolled_back_at: ISO 8601 timestamp of rollback, if applicable.
    """

    id: str
    command: str
    status: str
    working_dir: str = ""
    backup_path: Optional[str] = None
    created_at: Optional[str] = None
    rolled_back_at: Optional[str] = None

    @classmethod
    def from_dict(cls, data: Dict[str, Any]) -> "ExecutionResult":
        """Create an ExecutionResult from a dictionary."""
        return cls(
            id=data.get("id", ""),
            command=data.get("command", ""),
            status=data.get("status", ""),
            working_dir=data.get("workingDir", ""),
            backup_path=data.get("backupPath"),
            created_at=data.get("createdAt"),
            rolled_back_at=data.get("rolledBackAt"),
        )


class AttestClient:
    """Python client for the Attest verifiable agent action system.

    This client provides a Python interface to the Attest CLI, enabling
    programmatic management of agents, attestations, intents, and reversible
    executions.

    Attributes:
        cli_path: Path to the attest CLI executable.
        data_dir: Directory for Attest data storage.
        verbose: Whether to enable verbose output.

    Example:
        >>> client = AttestClient()
        >>> agent = client.agent_create(name="my-agent", agent_type="langchain")
        >>> attestations = client.attest_action(
        ...     agent_id=agent["id"],
        ...     action="command",
        ...     target="echo hello"
        ... )
    """

    DEFAULT_CLI_NAMES = ["attest", "attest.exe"]

    def __init__(
        self,
        cli_path: Optional[str] = None,
        data_dir: Optional[str] = None,
        verbose: bool = False,
    ):
        """Initialize the Attest client.

        Args:
            cli_path: Path to the attest CLI executable. If not provided,
                will search for 'attest' in PATH.
            data_dir: Directory for Attest data storage. Defaults to
                ~/.attest.
            verbose: Whether to enable verbose output.
        """
        self.cli_path = self._find_cli(cli_path)
        self.data_dir = data_dir or self._default_data_dir()
        self.verbose = verbose

    def _find_cli(self, cli_path: Optional[str] = None) -> str:
        """Find the attest CLI executable.

        Args:
            cli_path: Explicit path to the CLI.

        Returns:
            Path to the CLI executable.

        Raises:
            AttestConfigurationError: If CLI is not found.
        """
        if cli_path:
            if os.path.isfile(cli_path):
                return cli_path
            raise AttestConfigurationError(f"CLI not found at: {cli_path}")

        for name in self.DEFAULT_CLI_NAMES:
            path = self._search_path(name)
            if path:
                return path

        raise AttestConfigurationError(
            "Attest CLI not found. Please ensure 'attest' is installed and in PATH, "
            "or provide the cli_path parameter."
        )

    def _search_path(self, name: str) -> Optional[str]:
        """Search for an executable in PATH.

        Args:
            name: Name of the executable.

        Returns:
            Full path if found, None otherwise.
        """
        for directory in os.environ.get("PATH", "").split(os.pathsep):
            full_path = os.path.join(directory, name)
            if os.path.isfile(full_path) and os.access(full_path, os.X_OK):
                return full_path
        return None

    def _default_data_dir(self) -> str:
        """Get the default data directory.

        Returns:
            Path to the default data directory.
        """
        home = os.path.expanduser("~")
        return os.path.join(home, ".attest")

    def _run_command(
        self,
        args: List[str],
        input_data: Optional[str] = None,
        cwd: Optional[str] = None,
    ) -> subprocess.CompletedProcess:
        """Run an attest CLI command.

        Args:
            args: Arguments to pass to the CLI.
            input_data: Optional input data for the command.
            cwd: Working directory for the command.

        Returns:
            CompletedProcess with the command results.

        Raises:
            AttestCLIError: If the command fails.
        """
        env = os.environ.copy()
        if self.data_dir:
            env["ATTEST_DATA_DIR"] = self.data_dir

        full_args = [self.cli_path] + args
        if self.verbose:
            print(f"[attest] Running: {' '.join(full_args)}", file=sys.stderr)

        try:
            result = subprocess.run(
                full_args,
                input=input_data,
                capture_output=True,
                text=True,
                cwd=cwd,
                env=env,
                timeout=60,
            )
        except subprocess.TimeoutExpired:
            raise AttestCLIError("Command timed out", stderr="Timeout after 60 seconds")
        except FileNotFoundError:
            raise AttestCLIError(f"CLI not found at: {self.cli_path}")

        if result.returncode != 0:
            raise AttestCLIError(
                f"Command failed: {' '.join(args)}",
                return_code=result.returncode,
                stderr=result.stderr,
            )

        return result

    def _parse_json_output(self, output: str) -> Any:
        """Parse JSON output from the CLI.

        Args:
            output: JSON string to parse.

        Returns:
            Parsed JSON data.

        Raises:
            AttestCLIError: If output is not valid JSON.
        """
        output = output.strip()
        if not output:
            return None
        try:
            return json.loads(output)
        except json.JSONDecodeError as e:
            raise AttestCLIError(f"Invalid JSON output: {e}", stderr=output)

    def _ensure_json_flag(self, args: List[str]) -> List[str]:
        """Add --json flag to command arguments if not present.

        Args:
            args: Original arguments.

        Returns:
            Arguments with --json flag.
        """
        if "--json" not in args and "-json" not in args:
            return args + ["--json"]
        return args

    # Agent Management Methods

    def agent_create(
        self,
        name: str,
        agent_type: str = "generic",
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Agent:
        """Create a new agent identity.

        Args:
            name: Human-readable name for the agent.
            agent_type: Type of agent (e.g., "generic", "langchain", "autogen").
            metadata: Optional metadata dictionary for the agent.

        Returns:
            Agent object with the created agent's details.

        Raises:
            AttestCLIError: If agent creation fails.

        Example:
            >>> agent = client.agent_create(
            ...     name="my-assistant",
            ...     agent_type="langchain",
            ...     metadata={"model": "gpt-4"}
            ... )
        """
        args = ["agent", "create", "--name", name, "--type", agent_type, "--json"]
        if metadata:
            args.extend(["--meta", json.dumps(metadata)])

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Agent.from_dict(data)

    def agent_list(self, include_revoked: bool = False) -> List[Agent]:
        """List all registered agents.

        Args:
            include_revoked: Whether to include revoked agents.

        Returns:
            List of Agent objects.

        Example:
            >>> agents = client.agent_list()
            >>> for agent in agents:
            ...     print(f"{agent.id}: {agent.name}")
        """
        args = self._ensure_json_flag(["agent", "list"])
        if include_revoked:
            args.append("--all")

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)

        if isinstance(data, list):
            return [Agent.from_dict(item) for item in data]
        return []

    def agent_show(self, agent_id: str) -> Agent:
        """Show details of a specific agent.

        Args:
            agent_id: ID of the agent to show.

        Returns:
            Agent object with the agent's details.

        Raises:
            AttestNotFoundError: If agent is not found.
        """
        args = self._ensure_json_flag(["agent", "show", agent_id])
        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Agent.from_dict(data)

    def agent_delete(self, agent_id: str) -> bool:
        """Delete (revoke) an agent.

        Args:
            agent_id: ID of the agent to revoke.

        Returns:
            True if revocation was successful.

        Raises:
            AttestNotFoundError: If agent is not found.
        """
        args = ["agent", "delete", agent_id]
        self._run_command(args)
        return True

    def agent_export(self, agent_id: str) -> Dict[str, Any]:
        """Export an agent's public information.

        Args:
            agent_id: ID of the agent to export.

        Returns:
            Dictionary containing the agent's public information.

        Raises:
            AttestNotFoundError: If agent is not found.
        """
        args = self._ensure_json_flag(["agent", "export", agent_id])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    def agent_import(self, filepath: str) -> Dict[str, Any]:
        """Import an agent from a backup file.

        Args:
            filepath: Path to the JSON backup file.

        Returns:
            Dictionary with the imported agent's ID.

        Raises:
            AttestError: If import fails.
        """
        args = self._ensure_json_flag(["agent", "import", filepath])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    # Intent Management Methods

    def intent_create(
        self,
        goal: str,
        constraints: Optional[Dict[str, Any]] = None,
        acceptance_criteria: Optional[List[str]] = None,
        agent_id: Optional[str] = None,
    ) -> Intent:
        """Create a new intent record.

        Args:
            goal: The goal or objective of the intended action.
            constraints: Optional constraints on achieving the goal.
            acceptance_criteria: Optional criteria for accepting completion.
            agent_id: Optional ID of the agent associated with this intent.

        Returns:
            Intent object with the created intent's details.

        Example:
            >>> intent = client.intent_create(
            ...     goal="Refactor the authentication module",
            ...     constraints={"max_duration": "30min"},
            ...     acceptance_criteria=["All tests pass", "Code coverage > 80%"]
            ... )
        """
        args = self._ensure_json_flag(["intent", "create", goal])
        if constraints:
            args.extend(["--constraints", json.dumps(constraints)])
        if acceptance_criteria:
            args.extend(["--criteria", json.dumps(acceptance_criteria)])
        if agent_id:
            args.extend(["--agent", agent_id])

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Intent.from_dict(data)

    def intent_list(self, status: Optional[str] = None) -> List[Intent]:
        """List all intents.

        Args:
            status: Optional status filter (pending, active, completed, cancelled).

        Returns:
            List of Intent objects.
        """
        args = self._ensure_json_flag(["intent", "list"])
        if status:
            args.extend(["--status", status])

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)

        if isinstance(data, list):
            return [Intent.from_dict(item) for item in data]
        return []

    def intent_show(self, intent_id: str) -> Intent:
        """Show details of a specific intent.

        Args:
            intent_id: ID of the intent to show.

        Returns:
            Intent object with the intent's details.
        """
        args = self._ensure_json_flag(["intent", "show", intent_id])
        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Intent.from_dict(data)

    def intent_link_action(self, intent_id: str, action_id: str) -> bool:
        """Link an action attestation to an intent.

        Args:
            intent_id: ID of the intent.
            action_id: ID of the action attestation.

        Returns:
            True if linking was successful.
        """
        args = ["intent", "link", intent_id, action_id]
        self._run_command(args)
        return True

    # Attestation Methods

    def attest_action(
        self,
        agent_id: str,
        action: str,
        target: str,
        intent_id: Optional[str] = None,
        input_data: Optional[str] = None,
        session_id: Optional[str] = None,
    ) -> Attestation:
        """Create a cryptographic attestation for an agent action.

        Args:
            agent_id: ID of the agent creating this attestation.
            action: Type of action (e.g., "command", "file_edit", "api_call").
            target: Target of the action (command string, file path, etc.).
            intent_id: Optional ID of the associated intent.
            input_data: Optional input provided to the action.
            session_id: Optional session ID for grouping related actions.

        Returns:
            Attestation object with the attestation details.

        Raises:
            AttestNotFoundError: If agent is not found.
            AttestCLIError: If attestation creation fails.

        Example:
            >>> attestation = client.attest_action(
            ...     agent_id="aid:12345678",
            ...     action="command",
            ...     target="python script.py",
            ...     intent_id="int:abcdef"
            ... )
        """
        args = self._ensure_json_flag(
            [
                "attest",
                "create",
                "--agent",
                agent_id,
                "--action",
                action,
                "--target",
                target,
            ]
        )
        if intent_id:
            args.extend(["--intent", intent_id])
        if input_data:
            args.extend(["--input", input_data])
        if session_id:
            args.extend(["--session", session_id])

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Attestation.from_dict(data)

    def attest_list(
        self,
        agent_id: Optional[str] = None,
        intent_id: Optional[str] = None,
        limit: int = 20,
    ) -> List[Attestation]:
        """List attestations with optional filtering.

        Args:
            agent_id: Optional agent ID filter.
            intent_id: Optional intent ID filter.
            limit: Maximum number of results to return.

        Returns:
            List of Attestation objects.
        """
        args = self._ensure_json_flag(["attest", "list", "--limit", str(limit)])
        if agent_id:
            args.extend(["--agent", agent_id])
        if intent_id:
            args.extend(["--intent", intent_id])

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)

        if isinstance(data, list):
            return [Attestation.from_dict(item) for item in data]
        return []

    def attest_show(self, attestation_id: str) -> Attestation:
        """Show details of a specific attestation.

        Args:
            attestation_id: ID of the attestation to show.

        Returns:
            Attestation object with the attestation details.
        """
        args = self._ensure_json_flag(["attest", "show", attestation_id])
        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return Attestation.from_dict(data)

    def attest_export(self, attestation_id: str) -> Dict[str, Any]:
        """Export an attestation to JSON format.

        Args:
            attestation_id: ID of the attestation to export.

        Returns:
            Dictionary containing the attestation data.
        """
        args = self._ensure_json_flag(["attest", "export", attestation_id])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    def attest_import(self, filepath: str) -> Dict[str, Any]:
        """Import an attestation from a JSON file.

        Args:
            filepath: Path to the JSON file.

        Returns:
            Dictionary with the imported attestation's ID.
        """
        args = self._ensure_json_flag(["attest", "import", filepath])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    # Verification Methods

    def verify_attestation(self, attestation_id: str) -> Dict[str, Any]:
        """Verify the authenticity and integrity of an attestation.

        Args:
            attestation_id: ID of the attestation to verify.

        Returns:
            Dictionary containing verification results with keys:
            - valid: Whether the signature is valid
            - agent_id: ID of the agent that created the attestation
            - timestamp: When the attestation was created
            - details: Additional verification details

        Example:
            >>> result = client.verify_attestation("att:12345678")
            >>> if result["valid"]:
            ...     print("Attestation is valid!")
        """
        args = self._ensure_json_flag(["verify", "check", attestation_id])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    # Execution Methods

    def exec_run(
        self,
        command: str,
        reversible: bool = False,
        agent_id: Optional[str] = None,
        intent_id: Optional[str] = None,
        backup_type: str = "file",
        dry_run: bool = False,
    ) -> ExecutionResult:
        """Execute a command with optional reversibility.

        Args:
            command: The command to execute.
            reversible: Whether to make this execution reversible.
            agent_id: Optional agent ID for signing the execution.
            intent_id: Optional intent ID to link to this execution.
            backup_type: Type of backup (file, dir, none).
            dry_run: Whether to perform a dry run without executing.

        Returns:
            ExecutionResult object with execution details.

        Example:
            >>> result = client.exec_run(
            ...     command="python migrate.py",
            ...     reversible=True,
            ...     agent_id="aid:12345678"
            ... )
        """
        args = ["exec", "run"]
        if reversible:
            args.append("--reversible")
        if agent_id:
            args.extend(["--agent", agent_id])
        if intent_id:
            args.extend(["--intent", intent_id])
        if backup_type != "file":
            args.extend(["--backup", backup_type])
        if dry_run:
            args.append("--dry-run")

        args.append(command)

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return ExecutionResult.from_dict(data)

    def rollback(self, action_id: str = "last") -> ExecutionResult:
        """Rollback a reversible execution.

        Args:
            action_id: ID of the action to rollback, or "last" for the most recent.

        Returns:
            ExecutionResult object with rollback details.

        Example:
            >>> result = client.rollback("last")
        """
        args = self._ensure_json_flag(["exec", "rollback", action_id])
        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)
        return ExecutionResult.from_dict(data)

    def exec_history(self, pending_only: bool = False) -> List[ExecutionResult]:
        """Show execution history.

        Args:
            pending_only: Whether to show only pending rollbacks.

        Returns:
            List of ExecutionResult objects.
        """
        args = self._ensure_json_flag(["exec", "history"])
        if pending_only:
            args.append("--pending")

        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)

        if isinstance(data, list):
            return [ExecutionResult.from_dict(item) for item in data]
        return []

    # Policy Methods

    def policy_check(
        self,
        action: str,
        target: str,
        agent_id: Optional[str] = None,
    ) -> Dict[str, Any]:
        """Check if an action would be allowed by current policies.

        Args:
            action: Type of action to check.
            target: Target of the action.
            agent_id: Optional agent ID context.

        Returns:
            Dictionary containing:
            - allowed: Whether the action is allowed
            - reason: Reason for the decision
            - matching_policies: List of policies that were evaluated

        Example:
            >>> result = client.policy_check(
            ...     action="command",
            ...     target="rm -rf /"
            ... )
            >>> print(f"Allowed: {result['allowed']}")
        """
        args = self._ensure_json_flag(
            [
                "policy",
                "check",
                "--action",
                action,
                "--target",
                target,
            ]
        )
        if agent_id:
            args.extend(["--agent", agent_id])

        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    def policy_list(self) -> List[Dict[str, Any]]:
        """List all active policies.

        Returns:
            List of policy dictionaries.
        """
        args = self._ensure_json_flag(["policy", "list"])
        result = self._run_command(args)
        data = self._parse_json_output(result.stdout)

        if isinstance(data, list):
            return data
        return []

    def policy_add(self, filepath: str) -> Dict[str, Any]:
        """Add a policy from a YAML file.

        Args:
            filepath: Path to the YAML policy file.

        Returns:
            Dictionary with the added policy's ID.
        """
        args = self._ensure_json_flag(["policy", "add", filepath])
        result = self._run_command(args)
        return self._parse_json_output(result.stdout)

    def policy_remove(self, policy_id: str) -> bool:
        """Remove a policy by ID.

        Args:
            policy_id: ID of the policy to remove.

        Returns:
            True if removal was successful.
        """
        args = ["policy", "remove", policy_id]
        self._run_command(args)
        return True

    # Utility Methods

    def version(self) -> str:
        """Get the version of the Attest CLI.

        Returns:
            Version string.
        """
        result = self._run_command(["version"])
        return result.stdout.strip()

    def status(self) -> Dict[str, Any]:
        """Check the status of the Attest system.

        Returns:
            Dictionary containing system status information.
        """
        result = self._run_command(["status", "--json"])
        return self._parse_json_output(result.stdout)


def main_cli():
    """CLI entry point for the attest-agent package."""
    import argparse

    parser = argparse.ArgumentParser(
        description="Attest Agent SDK CLI",
        formatter_class=argparse.RawDescriptionHelpFormatter,
    )
    parser.add_argument(
        "--cli-path",
        help="Path to the attest CLI executable",
        default=None,
    )
    parser.add_argument(
        "--data-dir",
        help="Attest data directory",
        default=None,
    )
    parser.add_argument(
        "--verbose",
        "-v",
        help="Enable verbose output",
        action="store_true",
    )

    subparsers = parser.add_subparsers(dest="command", help="Available commands")

    agent_parser = subparsers.add_parser("agent", help="Agent management commands")
    agent_sub = agent_parser.add_subparsers(dest="agent_command")

    create_parser = agent_sub.add_parser("create", help="Create an agent")
    create_parser.add_argument("--name", required=True, help="Agent name")
    create_parser.add_argument("--type", default="generic", help="Agent type")
    create_parser.add_argument("--meta", help="JSON metadata")

    list_parser = agent_sub.add_parser("list", help="List agents")

    show_parser = agent_sub.add_parser("show", help="Show agent details")
    show_parser.add_argument("agent_id", help="Agent ID")

    args = parser.parse_args()

    try:
        client = AttestClient(
            cli_path=args.cli_path,
            data_dir=args.data_dir,
            verbose=args.verbose,
        )

        if args.command == "agent":
            if args.agent_command == "create":
                meta = json.loads(args.meta) if args.meta else None
                agent = client.agent_create(args.name, args.type, meta)
                print(json.dumps(agent.__dict__, indent=2))
            elif args.agent_command == "list":
                agents = client.agent_list()
                print(json.dumps([a.__dict__ for a in agents], indent=2))
            elif args.agent_command == "show":
                agent = client.agent_show(args.agent_id)
                print(json.dumps(agent.__dict__, indent=2))
            else:
                agent_parser.print_help()
        else:
            parser.print_help()
    except AttestError as e:
        print(f"Error: {e}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main_cli()
