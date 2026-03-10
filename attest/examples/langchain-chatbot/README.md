# LangChain + Attest Chatbot Example

This example demonstrates how to integrate Attest with a LangChain agent to automatically record and verify agent actions, track intent, and provide cryptographic attestations of all interactions.

## What You'll Learn

- How to create an agent identity in Attest
- How to use the `AttestCallbackHandler` for automatic action recording
- How to link agent actions to specific intents
- How to verify attestations after agent execution
- How to export and audit agent session data

## Prerequisites

1. **Attest CLI installed** - See [Attest Installation](https://github.com/attest/attest#installation)
2. **Python 3.10+** 
3. **OpenAI API key** (or another LLM provider)

## Quick Start

### 1. Install Dependencies

```bash
cd examples/langchain-chatbot
pip install -r requirements.txt
```

### 2. Configure Environment

Copy the example environment file and fill in your values:

```bash
cp .env.example .env
```

Edit `.env` with your configuration:
- `OPENAI_API_KEY` - Your OpenAI API key
- `ATTEST_DATA_DIR` - Path to Attest data directory (optional)

### 3. Initialize Attest

If you haven't already, initialize Attest in your project:

```bash
# From the project root
attest init

# Create an agent identity for this chatbot
attest agent create --name "langchain-chatbot" --type "langchain" --json
```

### 4. Run the Example

```bash
python chatbot.py
```

## How It Works

### Agent Identity Creation

The chatbot creates a cryptographically verifiable agent identity:

```python
agent = client.agent_create(
    name="langchain-chatbot",
    agent_type="langchain",
    metadata={"model": "gpt-4", "version": "1.0"}
)
```

This identity is used to sign all subsequent actions, providing non-repudiation.

### Intent Tracking

Before executing, the chatbot creates an intent record that captures:

- **Goal**: What the agent intends to accomplish
- **Constraints**: Any limitations on how to achieve the goal
- **Acceptance Criteria**: How to determine if the goal was met

```python
intent = client.intent_create(
    goal=goal,
    constraints={"max_steps": 10, "safety_level": "high"},
    acceptance_criteria=["Question answered accurately", "Sources cited"]
)
```

### Automatic Action Recording

The `AttestCallbackHandler` automatically records:

1. **LLM Prompts & Responses** - Every message sent to and from the LLM
2. **Tool Calls** - When the agent invokes tools (search, calculator, etc.)
3. **Chain Execution** - Start and end of chain execution
4. **Agent Thoughts** - Reasoning steps taken by the agent
5. **Errors** - Any errors that occur during execution

```python
handler = AttestCallbackHandler(
    agent_id=agent.id,
    intent_id=intent.id,
    verbose=True
)
callback_manager = CallbackManager([handler])
```

### Verification

After execution, you can verify the authenticity of all recorded actions:

```python
# Get all attestations for this session
attestations = client.attest_list(intent_id=intent.id)

for att in attestations:
    result = client.verify_attestation(att.id)
    print(f"Attestation {att.id}: {result['valid']}")
```

## Example Output

```
=== Attest LangChain Chatbot ===
[INFO] Creating agent identity...
[INFO] Agent created: aid:7f8d9e2a (langchain-chatbot)
[INFO] Creating intent: Answer user question about quantum computing
[INFO] Intent created: int:a1b2c3d4

=== Agent Execution ===
[attest-cb] LLM start: You are a helpful assistant...
[attest-cb] Tool start: search
[attest-cb] Tool end: search
[attest-cb] Agent finish: {'output': 'Quantum computing is...'}

=== Session Summary ===
Session ID: abc12345
Agent: langchain-chatbot (aid:7f8d9e2a)
Intent: Answer user question about quantum computing (int:a1b2c3d4)
Total Actions: 8
Tool Calls: 2

=== Verification ===
Verifying attestation att:12345678... VALID
Verifying attestation att:12345679... VALID
Verifying attestation att:12345680... VALID

All 8 attestations verified successfully!
```

## Project Structure

```
langchain-chatbot/
├── README.md          # This file
├── chatbot.py         # Main example application
├── requirements.txt   # Python dependencies
└── .env.example       # Environment variables template
```

## Code Overview

### Main Components

1. **`create_agent()`** - Creates a new agent identity or retrieves existing one
2. **`create_intent()`** - Records the goal and constraints for the session
3. **`setup_callback()`** - Configures the Attest callback handler
4. **`run_chatbot()`** - Executes the agent with Attest monitoring
5. **`verify_session()`** - Verifies all recorded attestations
6. **`export_session()`** - Exports session data for auditing

### Key Attest Concepts

- **Agent Identity (AID)**: Cryptographically generated ID tied to a keypair
- **Intent**: Records WHY the agent is doing something, not just WHAT
- **Attestation**: Cryptographically signed record of each action
- **Session**: Groups related actions together for easier auditing

## Troubleshooting

### "Attest CLI not found"

Ensure Attest is installed and in your PATH:
```bash
which attest  # Linux/Mac
where attest  # Windows
```

### "Agent not found"

The agent may have been deleted. Create a new one:
```bash
attest agent create --name "langchain-chatbot" --type "langchain"
```

### Verification fails

The attestation data may be corrupted or tampered with. Check the data directory:
```bash
attest status
```

## Next Steps

- Explore the [AutoGen Team example](../autogen-team/) for multi-agent scenarios
- Check out the [CrewAI Research example](../crewai-research/) for task automation
- Read the [Attest Documentation](https://github.com/attest/attest) for deep dives
