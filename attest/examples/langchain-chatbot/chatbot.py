"""
LangChain + Attest Chatbot Example

This module demonstrates how to integrate Attest with a LangChain agent to create
a verifiable chatbot with automatic action recording, intent tracking, and
cryptographic attestations.

Features:
- Automatic recording of all agent actions (LLM calls, tool usage, reasoning)
- Intent tracking to capture WHY the agent is taking actions
- Cryptographic signing of each action for non-repudiation
- Session export for audit and compliance
- Verification of attestations after execution

Usage:
    python chatbot.py
"""

import os
import sys
import json
from datetime import datetime
from typing import Dict, List, Optional, Any

from dotenv import load_dotenv

from langchain_openai import ChatOpenAI
from langchain.agents import AgentExecutor, create_openai_functions_agent
from langchain.prompts import ChatPromptTemplate, MessagesPlaceholder
from langchain.callbacks import CallbackManager
from langchain.schema import HumanMessage, SystemMessage

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "sdk", "python"))

from attest_client import AttestClient, AttestError, Agent, Intent, Attestation
from langchain_callback import AttestCallbackHandler


load_dotenv()


def print_header(title: str) -> None:
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def print_info(message: str) -> None:
    """Print an informational message."""
    print(f"[INFO] {message}")


def print_success(message: str) -> None:
    """Print a success message."""
    print(f"[SUCCESS] {message}")


def print_error(message: str) -> None:
    """Print an error message."""
    print(f"[ERROR] {message}")


def create_agent(
    client: AttestClient, name: str, agent_type: str = "langchain"
) -> Agent:
    """
    Create a new agent identity in Attest.

    An agent identity is a cryptographically verifiable entity that signs all
    actions taken by this chatbot. Each agent has a unique ID (AID) tied to
    a public/private keypair.

    Args:
        client: AttestClient instance
        name: Human-readable name for the agent
        agent_type: Type of agent (langchain, autogen, crewai, etc.)

    Returns:
        Agent object with id, name, type, and public_key
    """
    print_info(f"Creating agent identity: {name}")

    try:
        agent = client.agent_create(
            name=name,
            agent_type=agent_type,
            metadata={
                "model": "gpt-4",
                "version": "1.0.0",
                "created_at": datetime.utcnow().isoformat(),
            },
        )
        print_success(f"Agent created: {agent.name} ({agent.id})")
        return agent
    except AttestError as e:
        print_error(f"Failed to create agent: {e}")
        raise


def get_or_create_agent(
    client: AttestClient, name: str, agent_type: str = "langchain"
) -> Agent:
    """
    Get existing agent or create a new one.

    Useful for reusing the same agent across multiple sessions.
    """
    try:
        agents = client.agent_list()
        for agent in agents:
            if agent.name == name and not agent.revoked:
                print_info(f"Found existing agent: {name} ({agent.id})")
                return agent

        return create_agent(client, name, agent_type)
    except AttestError:
        return create_agent(client, name, agent_type)


def create_intent(
    client: AttestClient,
    goal: str,
    constraints: Optional[Dict[str, Any]] = None,
    acceptance_criteria: Optional[List[str]] = None,
) -> Intent:
    """
    Create an intent record for the chatbot session.

    Intents capture WHY the agent is taking actions, not just WHAT it does.
    This is crucial for understanding agent decision-making and for
    audit/compliance purposes.

    Args:
        client: AttestClient instance
        goal: The objective the agent should achieve
        constraints: Limitations on how to achieve the goal
        acceptance_criteria: Criteria for determining goal completion

    Returns:
        Intent object with id, goal, status, and timestamps
    """
    print_info(f"Creating intent: {goal[:50]}...")

    default_constraints = {
        "max_steps": 10,
        "safety_level": "high",
        "require_sources": True,
    }
    constraints = {**default_constraints, **(constraints or {})}

    default_criteria = [
        "Question answered accurately",
        "Response is helpful and relevant",
        "Sources cited where applicable",
    ]
    acceptance_criteria = acceptance_criteria or default_criteria

    intent = client.intent_create(
        goal=goal, constraints=constraints, acceptance_criteria=acceptance_criteria
    )

    print_success(f"Intent created: {intent.id}")
    print(f"  Goal: {intent.goal}")
    print(f"  Constraints: {json.dumps(intent.constraints, indent=2)}")

    return intent


def setup_callback(
    agent: Agent, intent: Intent, verbose: bool = True
) -> tuple[AttestCallbackHandler, CallbackManager]:
    """
    Set up the Attest callback handler for LangChain.

    The AttestCallbackHandler automatically records:
    - LLM prompts and responses
    - Tool invocations and outputs
    - Chain execution start/end
    - Agent reasoning and thoughts
    - Errors and exceptions

    Args:
        agent: Agent identity to use for signing
        intent: Intent to link actions to
        verbose: Enable verbose output

    Returns:
        Tuple of (handler, callback_manager)
    """
    print_info("Setting up Attest callback handler...")

    handler = AttestCallbackHandler(
        agent_id=agent.id, intent_id=intent.id, verbose=verbose
    )

    callback_manager = CallbackManager([handler])

    print_success("Callback handler configured")

    return handler, callback_manager


def create_chatbot(llm, tools, callback_manager) -> AgentExecutor:
    """
    Create the LangChain agent executor with Attest monitoring.

    Args:
        llm: Language model instance
        tools: List of tools available to the agent
        callback_manager: Attest callback manager

    Returns:
        Configured AgentExecutor ready for execution
    """
    prompt = ChatPromptTemplate.from_messages(
        [
            SystemMessage(
                content="""You are a helpful, honest AI assistant.
You must always be accurate and cite your sources when providing factual information.
If you don't know something, say so clearly. Always think step by step."""
            ),
            MessagesPlaceholder(variable_name="chat_history", optional=True),
            HumanMessage(content="{input}"),
            MessagesPlaceholder(variable_name="agent_scratchpad"),
        ]
    )

    agent = create_openai_functions_agent(llm, tools, prompt)
    executor = AgentExecutor(
        agent=agent,
        tools=tools,
        callback_manager=callback_manager,
        verbose=True,
        max_iterations=10,
    )

    return executor


def run_chatbot(
    executor: AgentExecutor, user_input: str, chat_history: Optional[List] = None
) -> Dict[str, Any]:
    """
    Run the chatbot with user input.

    This function executes the agent and returns the result along with
    timing information for performance monitoring.

    Args:
        executor: Configured AgentExecutor
        user_input: User's question or request
        chat_history: Previous conversation history

    Returns:
        Dictionary with response, success status, and timing
    """
    print_header("Agent Execution")
    print(f"User: {user_input}\n")

    start_time = datetime.utcnow()

    try:
        result = executor.invoke(
            {"input": user_input, "chat_history": chat_history or []}
        )

        end_time = datetime.utcnow()
        duration = (end_time - start_time).total_seconds()

        print(f"\nAssistant: {result['output']}")
        print(f"\n[Execution time: {duration:.2f}s]")

        return {"success": True, "output": result["output"], "duration": duration}

    except Exception as e:
        end_time = datetime.utcnow()
        duration = (end_time - start_time).total_seconds()

        print_error(f"Agent execution failed: {e}")

        return {
            "success": False,
            "output": str(e),
            "duration": duration,
            "error": str(e),
        }


def verify_attestations(client: AttestClient, intent_id: str) -> Dict[str, Any]:
    """
    Verify all attestations created during the session.

    Verification ensures that:
    1. The attestation was created by the claimed agent
    2. The signature is valid
    3. The data has not been tampered with

    Args:
        client: AttestClient instance
        intent_id: ID of the intent to get attestations for

    Returns:
        Dictionary with verification results summary
    """
    print_header("Verification")

    attestations = client.attest_list(intent_id=intent_id)

    print(f"Found {len(attestations)} attestations to verify\n")

    verified_count = 0
    failed_count = 0

    for att in attestations:
        try:
            result = client.verify_attestation(att.id)

            if result.get("valid", False):
                print(f"  [✓] {att.action_type}: {att.action_target[:40]}...")
                verified_count += 1
            else:
                print(f"  [✗] {att.action_type}: {att.action_target[:40]}... FAILED")
                failed_count += 1

        except Exception as e:
            print(f"  [✗] {att.action_type}: {att.action_target[:40]}... ERROR: {e}")
            failed_count += 1

    print(f"\nVerification complete: {verified_count} valid, {failed_count} failed")

    return {
        "total": len(attestations),
        "verified": verified_count,
        "failed": failed_count,
    }


def export_session_data(
    client: AttestClient,
    agent: Agent,
    intent: Intent,
    handler: AttestCallbackHandler,
    execution_result: Dict[str, Any],
) -> Dict[str, Any]:
    """
    Export the complete session data for audit purposes.

    The exported data includes:
    - Agent identity and public key
    - Intent details and constraints
    - All attestations with signatures
    - Execution timing and results
    - Callback handler session data

    Args:
        client: AttestClient instance
        agent: Agent identity
        intent: Session intent
        handler: Callback handler with recorded data
        execution_result: Result from chatbot execution

    Returns:
        Complete session export dictionary
    """
    print_header("Session Export")

    attestations = client.attest_list(intent_id=intent.id)

    session_data = {
        "exported_at": datetime.utcnow().isoformat(),
        "version": "1.0",
        "agent": {
            "id": agent.id,
            "name": agent.name,
            "type": agent.type,
            "public_key": agent.public_key,
            "created_at": agent.created_at,
        },
        "intent": {
            "id": intent.id,
            "goal": intent.goal,
            "constraints": intent.constraints,
            "acceptance_criteria": intent.acceptance_criteria,
            "status": intent.status,
            "created_at": intent.created_at,
        },
        "execution": {
            "success": execution_result["success"],
            "duration_seconds": execution_result.get("duration", 0),
            "error": execution_result.get("error"),
        },
        "attestations": {"total": len(attestations), "by_type": {}},
        "callback_session": handler.export_session(),
    }

    for att in attestations:
        att_type = att.action_type
        if att_type not in session_data["attestations"]["by_type"]:
            session_data["attestations"]["by_type"][att_type] = 0
        session_data["attestations"]["by_type"][att_type] += 1

    export_path = os.path.join(os.path.dirname(__file__), "session_export.json")

    with open(export_path, "w") as f:
        json.dump(session_data, f, indent=2, default=str)

    print_success(f"Session exported to: {export_path}")
    print(f"  Agent: {agent.name} ({agent.id})")
    print(f"  Intent: {intent.id}")
    print(f"  Attestations: {len(attestations)}")
    print(f"  Execution: {'Success' if execution_result['success'] else 'Failed'}")

    return session_data


def demo_conversation(client: AttestClient) -> None:
    """
    Run a demonstration conversation showing full Attest integration.

    This function demonstrates:
    1. Agent identity creation/retrieval
    2. Intent creation with goals and constraints
    3. Callback handler setup
    4. Chatbot execution with various query types
    5. Verification of all attestations
    6. Session export for audit
    """
    print_header("Attest LangChain Chatbot Demo")
    print("This demo shows how Attest provides verifiable agent actions.\n")

    agent = get_or_create_agent(client, "langchain-chatbot", "langchain")

    conversation_topics = [
        {
            "goal": "Answer user question about quantum computing",
            "question": "What is quantum computing and how does it differ from classical computing?",
            "criteria": [
                "Clear explanation of quantum computing",
                "Comparison with classical computing",
            ],
        },
        {
            "goal": "Explain climate change impacts",
            "question": "What are the main impacts of climate change on ocean ecosystems?",
            "criteria": ["Accurate scientific information", "Specific examples given"],
        },
    ]

    for topic in conversation_topics:
        print_header(f"Topic: {topic['goal']}")

        intent = create_intent(
            client, goal=topic["goal"], acceptance_criteria=topic["criteria"]
        )

        handler, callback_manager = setup_callback(agent, intent, verbose=True)

        llm = ChatOpenAI(
            model="gpt-4", api_key=os.getenv("OPENAI_API_KEY"), temperature=0
        )

        from langchain_community.utilities import SerpAPIWrapper
        from langchain.prompts import PromptTemplate
        from langchain.tools import Tool

        search = SerpAPIWrapper()

        tools = [
            Tool(
                name="search",
                description="Search for information on the web",
                func=search.run,
            )
        ]

        executor = create_chatbot(llm, tools, callback_manager)

        execution_result = run_chatbot(executor, topic["question"])

        if execution_result["success"]:
            verify_attestations(client, intent.id)
            export_session_data(client, agent, intent, handler, execution_result)
        else:
            print_error("Skipping verification due to execution failure")

        print()


def main() -> None:
    """Main entry point for the chatbot demo."""
    print_header("Attest + LangChain Integration Demo")

    try:
        client = AttestClient(verbose=True)
        print_success("Attest client initialized")
        print(f"  CLI path: {client.cli_path}")
        print(f"  Data directory: {client.data_dir}")

        status = client.status()
        print(f"  Status: {status.get('initialized', 'unknown')}")

    except AttestError as e:
        print_error(f"Failed to initialize Attest client: {e}")
        print("\nPlease ensure Attest is installed and initialized:")
        print("  1. Install Attest: go install github.com/attest/attest")
        print("  2. Initialize: attest init")
        print("  3. Create agent: attest agent create --name chatbot --type langchain")
        sys.exit(1)

    if not os.getenv("OPENAI_API_KEY"):
        print_error("OPENAI_API_KEY not set in environment")
        print("Please create a .env file with your API key:")
        print("  echo 'OPENAI_API_KEY=your-key' > .env")
        sys.exit(1)

    try:
        demo_conversation(client)

        print_header("Demo Complete")
        print("What we demonstrated:")
        print("  1. Agent identity creation - cryptographically verifiable identity")
        print("  2. Intent tracking - captures WHY the agent acts")
        print("  3. Automatic action recording - all LLM calls and tools logged")
        print("  4. Cryptographic attestations - every action signed")
        print("  5. Verification - confirm authenticity of all actions")
        print("  6. Session export - complete audit trail for compliance")

        print("\nNext steps:")
        print("  - Explore the AutoGen team example for multi-agent scenarios")
        print("  - Check the CrewAI research example for task automation")
        print("  - Read the full Attest documentation for advanced features")

    except KeyboardInterrupt:
        print("\n\nDemo interrupted by user")
        sys.exit(0)
    except Exception as e:
        print_error(f"Demo failed: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
