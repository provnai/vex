"""LangChain callback handler for automatic attestation recording.

This module provides a callback handler that automatically records
LangChain agent actions, tool calls, and thoughts to the Attest system.

Example:
    >>> from langchain.callbacks import CallbackManager
    >>> from langchain_callback import AttestCallbackHandler
    >>> handler = AttestCallbackHandler(
    ...     agent_id="aid:12345678",
    ...     intent_id="int:abcdef"
    ... )
    >>> callback_manager = CallbackManager([handler])
    >>> agent = Agent(..., callback_manager=callback_manager)
    >>> agent.run("Analyze this data")
"""

from typing import Any, Dict, List, Optional, Union
import threading
import time
import uuid

from langchain.callbacks.base import BaseCallbackHandler
from langchain.schema import AgentAction, AgentFinish, LLMResult

try:
    from langchain.schema import AgentThought

    AGENT_THOUGHTS_AVAILABLE = True
except ImportError:
    AGENT_THOUGHTS_AVAILABLE = False


class AttestCallbackHandler(BaseCallbackHandler):
    """Callback handler for automatic attestation of LangChain agent actions.

    This handler integrates with LangChain's callback system to automatically
    record agent actions, tool calls, and thoughts to the Attest system,
    providing cryptographic attestations of agent behavior.

    Attributes:
        agent_id: ID of the agent identity to use for signing.
        intent_id: Optional intent ID to link actions to.
        client: Optional AttestClient instance.
        session_id: Unique session identifier for grouping actions.
        verbose: Whether to print verbose output.

    Example:
        >>> handler = AttestCallbackHandler(
        ...     agent_id="aid:12345678",
        ...     intent_id="int:abcdef",
        ...     verbose=True
        ... )
        >>> agent.run("Your task here")
    """

    def __init__(
        self,
        agent_id: str,
        intent_id: Optional[str] = None,
        client: Optional["AttestClient"] = None,
        session_id: Optional[str] = None,
        verbose: bool = False,
    ):
        """Initialize the Attest callback handler.

        Args:
            agent_id: ID of the agent identity to use for signing.
            intent_id: Optional intent ID to link actions to.
            client: Optional AttestClient instance. If not provided,
                a new client will be created.
            session_id: Optional session ID for grouping actions.
            verbose: Whether to print verbose output.
        """
        super().__init__()
        self.agent_id = agent_id
        self.intent_id = intent_id
        self.session_id = session_id or str(uuid.uuid4())[:8]
        self.verbose = verbose
        self._lock = threading.Lock()
        self._action_count = 0

        if client is None:
            from attest_client import AttestClient

            self.client = AttestClient()
        else:
            self.client = client

        self._actions: List[Dict[str, Any]] = []
        self._thoughts: List[Dict[str, Any]] = []
        self._tool_calls: List[Dict[str, Any]] = []

    def _print(self, message: str) -> None:
        """Print message if verbose mode is enabled."""
        if self.verbose:
            print(f"[attest-cb] {message}", flush=True)

    def _create_attestation(
        self,
        action_type: str,
        target: str,
        input_data: Optional[str] = None,
        metadata: Optional[Dict[str, Any]] = None,
    ) -> Optional[Dict[str, Any]]:
        """Create an attestation for an action.

        Args:
            action_type: Type of action (e.g., "tool_call", "llm_prompt").
            target: Target of the action (tool name, prompt, etc.).
            input_data: Optional input data.
            metadata: Optional additional metadata.

        Returns:
            Attestation result or None if it fails.
        """
        try:
            self._action_count += 1
            result = self.client.attest_action(
                agent_id=self.agent_id,
                action=action_type,
                target=target,
                intent_id=self.intent_id,
                input_data=input_data,
                session_id=self.session_id,
            )
            self._print(f"Attested: {action_type} -> {target}")
            return {
                "attestation_id": result.id,
                "action_type": action_type,
                "target": target,
                "timestamp": result.timestamp,
                "signature": result.signature,
            }
        except Exception as e:
            self._print(f"Failed to create attestation: {e}")
            return None

    def _record_thought(
        self,
        thought: str,
        step: int,
        thought_type: str = "reasoning",
    ) -> None:
        """Record an agent thought.

        Args:
            thought: The thought content.
            step: The step number.
            thought_type: Type of thought (reasoning, plan, observation).
        """
        thought_record = {
            "id": str(uuid.uuid4()),
            "thought": thought,
            "step": step,
            "type": thought_type,
            "timestamp": time.time(),
        }
        with self._lock:
            self._thoughts.append(thought_record)
        self._create_attestation(
            action_type="thought",
            target=f"step_{step}_{thought_type}",
            input_data=thought[:500] if len(thought) > 500 else thought,
            metadata={"thought_type": thought_type, "step": step},
        )

    def _record_tool_call(
        self,
        tool: str,
        input_args: Dict[str, Any],
        output: Optional[str] = None,
        step: int = 0,
    ) -> None:
        """Record a tool call.

        Args:
            tool: Name of the tool.
            input_args: Arguments passed to the tool.
            output: Output from the tool.
            step: The step number.
        """
        tool_record = {
            "id": str(uuid.uuid4()),
            "tool": tool,
            "input": input_args,
            "output": output,
            "step": step,
            "timestamp": time.time(),
        }
        with self._lock:
            self._tool_calls.append(tool_record)

        input_str = str(input_args)[:500]
        self._create_attestation(
            action_type="tool_call",
            target=tool,
            input_data=input_str,
            metadata={"step": step, "tool": tool},
        )

    def on_llm_start(
        self,
        serialized: Dict[str, Any],
        prompts: List[str],
        **kwargs: Any,
    ) -> None:
        """Called when LLM starts processing.

        Args:
            serialized: Serialized LLM configuration.
            prompts: List of prompts being processed.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"LLM start: {prompts[0][:50]}..." if prompts else "LLM start")
        for i, prompt in enumerate(prompts):
            self._create_attestation(
                action_type="llm_prompt",
                target=serialized.get("name", "unknown_llm"),
                input_data=prompt[:1000] if prompt else "",
                metadata={"prompt_index": i, "total_prompts": len(prompts)},
            )

    def on_llm_end(self, response: LLMResult, **kwargs: Any) -> None:
        """Called when LLM finishes processing.

        Args:
            response: LLM response result.
            **kwargs: Additional keyword arguments.
        """
        self._print("LLM end")
        if response.generations:
            gen = response.generations[0][0] if response.generations[0] else None
            if gen:
                output = getattr(gen, "text", str(gen))[:1000]
                self._create_attestation(
                    action_type="llm_response",
                    target=serialized.get("name", "unknown_llm")
                    if "serialized" in kwargs
                    else "llm",
                    input_data=output,
                    metadata={"response_length": len(output)},
                )

    def on_llm_error(
        self,
        error: Union[Exception, KeyboardInterrupt],
        **kwargs: Any,
    ) -> None:
        """Called when LLM errors.

        Args:
            error: The error that occurred.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"LLM error: {error}")
        self._create_attestation(
            action_type="llm_error",
            target="error",
            input_data=str(error)[:500],
            metadata={"error_type": type(error).__name__},
        )

    def on_chain_start(
        self,
        serialized: Dict[str, Any],
        inputs: Dict[str, Any],
        **kwargs: Any,
    ) -> None:
        """Called when chain starts executing.

        Args:
            serialized: Serialized chain configuration.
            inputs: Chain inputs.
            **kwargs: Additional keyword arguments.
        """
        chain_name = serialized.get("name", serialized.get("id", "unknown"))
        self._print(f"Chain start: {chain_name}")
        self._create_attestation(
            action_type="chain_start",
            target=chain_name,
            input_data=str(inputs)[:500],
            metadata={"chain_name": chain_name},
        )

    def on_chain_end(self, outputs: Dict[str, Any], **kwargs: Any) -> None:
        """Called when chain finishes executing.

        Args:
            outputs: Chain outputs.
            **kwargs: Additional keyword arguments.
        """
        self._print("Chain end")
        self._create_attestation(
            action_type="chain_end",
            target="completion",
            input_data=str(outputs)[:500],
            metadata={},
        )

    def on_chain_error(
        self,
        error: Union[Exception, KeyboardInterrupt],
        **kwargs: Any,
    ) -> None:
        """Called when chain errors.

        Args:
            error: The error that occurred.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"Chain error: {error}")
        self._create_attestation(
            action_type="chain_error",
            target="error",
            input_data=str(error)[:500],
            metadata={"error_type": type(error).__name__},
        )

    def on_tool_start(
        self,
        serialized: Dict[str, Any],
        input_str: str,
        **kwargs: Any,
    ) -> None:
        """Called when tool starts executing.

        Args:
            serialized: Serialized tool configuration.
            input_str: Input string to the tool.
            **kwargs: Additional keyword arguments.
        """
        tool_name = serialized.get("name", "unknown")
        self._print(f"Tool start: {tool_name}")
        self._create_attestation(
            action_type="tool_start",
            target=tool_name,
            input_data=input_str[:500],
            metadata={"tool_name": tool_name},
        )

    def on_tool_end(
        self,
        output: str,
        **kwargs: Any,
    ) -> None:
        """Called when tool finishes executing.

        Args:
            output: Tool output.
            **kwargs: Additional keyword arguments.
        """
        tool_name = kwargs.get("name", "unknown")
        self._print(f"Tool end: {tool_name}")
        self._create_attestation(
            action_type="tool_end",
            target=tool_name,
            input_data=output[:1000] if output else "",
            metadata={},
        )

    def on_tool_error(
        self,
        error: Union[Exception, KeyboardInterrupt],
        **kwargs: Any,
    ) -> None:
        """Called when tool errors.

        Args:
            error: The error that occurred.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"Tool error: {error}")
        tool_name = kwargs.get("name", "unknown")
        self._create_attestation(
            action_type="tool_error",
            target=tool_name,
            input_data=str(error)[:500],
            metadata={"error_type": type(error).__name__, "tool_name": tool_name},
        )

    def on_agent_action(
        self,
        action: AgentAction,
        **kwargs: Any,
    ) -> None:
        """Called when agent takes an action.

        Args:
            action: The agent action.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"Agent action: {action.tool} -> {action.tool_input}")
        self._record_tool_call(
            tool=action.tool,
            input_args=action.tool_input
            if isinstance(action.tool_input, dict)
            else {"input": action.tool_input},
            step=kwargs.get("step", self._action_count),
        )

    def on_agent_finish(
        self,
        finish: AgentFinish,
        **kwargs: Any,
    ) -> None:
        """Called when agent finishes.

        Args:
            finish: The agent finish result.
            **kwargs: Additional keyword arguments.
        """
        self._print(f"Agent finish: {finish.return_values}")
        self._create_attestation(
            action_type="agent_finish",
            target="completion",
            input_data=str(finish.return_values)[:500],
            metadata={"output_keys": list(finish.return_values.keys())},
        )

    def on_text(self, text: str, **kwargs: Any) -> None:
        """Called when text is printed during execution.

        Args:
            text: The printed text.
            **kwargs: Additional keyword arguments.
        """
        if self.verbose:
            print(f"[text] {text}")

    def on_agent_thought(self, thought: "AgentThought", **kwargs: Any) -> None:
        """Called when agent has a thought (if supported).

        Args:
            thought: The agent thought.
            **kwargs: Additional keyword arguments.
        """
        if not AGENT_THOUGHTS_AVAILABLE:
            return
        self._print(f"Thought: {thought.thought[:50]}...")
        self._record_thought(
            thought=thought.thought,
            step=thought.step,
            thought_type=getattr(thought, "type", "reasoning"),
        )

    def get_session_summary(self) -> Dict[str, Any]:
        """Get a summary of the recorded session.

        Returns:
            Dictionary containing session summary with:
            - session_id: The session identifier
            - agent_id: The agent ID used
            - intent_id: The linked intent ID, if any
            - action_count: Total number of actions recorded
            - tool_calls: List of tool calls
            - thoughts: List of thoughts
            - attestations: Summary of attestations created
        """
        with self._lock:
            return {
                "session_id": self.session_id,
                "agent_id": self.agent_id,
                "intent_id": self.intent_id,
                "action_count": self._action_count,
                "tool_calls": list(self._tool_calls),
                "thoughts": list(self._thoughts),
                "attestations": {
                    "total": self._action_count,
                    "verified": sum(
                        1 for a in self._actions if a.get("verified", False)
                    ),
                },
            }

    def export_session(self) -> Dict[str, Any]:
        """Export the full session data.

        Returns:
            Complete session data including all recorded actions and metadata.
        """
        summary = self.get_session_summary()
        return {
            **summary,
            "exported_at": time.strftime("%Y-%m-%dT%H:%M:%SZ", time.gmtime()),
            "version": "1.0",
        }


def create_attest_callback(
    agent_id: str,
    intent_id: Optional[str] = None,
    verbose: bool = False,
) -> AttestCallbackHandler:
    """Factory function to create an Attest callback handler.

    This is a convenience function for quickly creating callback handlers
    with typical settings.

    Args:
        agent_id: ID of the agent identity to use.
        intent_id: Optional intent ID to link actions to.
        verbose: Whether to enable verbose output.

    Returns:
        Configured AttestCallbackHandler instance.

    Example:
        >>> handler = create_attest_callback(
        ...     agent_id="aid:12345678",
        ...     intent_id="int:abcdef",
        ...     verbose=True
        ... )
        >>> # Use with any LangChain agent
        >>> agent.run("Your task")
    """
    return AttestCallbackHandler(
        agent_id=agent_id,
        intent_id=intent_id,
        verbose=verbose,
    )


def create_managed_callback(
    agent_id: str,
    intent_id: Optional[str] = None,
    verbose: bool = False,
) -> tuple[AttestCallbackHandler, "CallbackManager"]:
    """Create a callback handler with its own callback manager.

    This is useful when you want a self-contained setup with proper
    callback manager integration.

    Args:
        agent_id: ID of the agent identity to use.
        intent_id: Optional intent ID to link actions to.
        verbose: Whether to enable verbose output.

    Returns:
        Tuple of (handler, callback_manager).

    Example:
        >>> handler, manager = create_managed_callback(
        ...     agent_id="aid:12345678",
        ...     verbose=True
        ... )
        >>> agent = OpenAIAgent(..., callbacks=manager)
    """
    from langchain.callbacks import CallbackManager

    handler = create_attest_callback(agent_id, intent_id, verbose)
    manager = CallbackManager([handler])
    return handler, manager
