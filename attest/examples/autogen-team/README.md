# AutoGen + Attest Multi-Agent Team Example

This example demonstrates how to integrate Attest with AutoGen to create a verifiable multi-agent team where agent-to-agent interactions are cryptographically attested and can be audited for compliance.

## What You'll Learn

- How to create agent identities for multiple agents in a team
- How to link agent-to-agent attestations for collaborative workflows
- How to track team intent across multiple agents
- How to verify the complete chain of agent interactions
- How to audit multi-agent conversations for compliance

## Prerequisites

1. **Attest CLI installed** - See [Attest Installation](https://github.com/attest/attest#installation)
2. **Python 3.10+**
3. **OpenAI API key** (or another LLM provider)

## Quick Start

### 1. Install Dependencies

```bash
cd examples/autogen-team
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

# Create agent identities for each team member
attest agent create --name "researcher" --type "autogen" --json
attest agent create --name "analyst" --type "autogen" --json
attest agent create --name "writer" --type "autogen" --json
```

### 4. Run the Example

```bash
python team.py
```

## How It Works

### Team Agent Creation

Each team member gets their own cryptographically verifiable identity:

```python
agents = {}
for name, role in [("researcher", "research"), ("analyst", "analysis"), ("writer", "writing")]:
    agents[name] = client.agent_create(
        name=name,
        agent_type="autogen",
        metadata={"role": role}
    )
```

### Team Intent

The entire team works toward a shared intent:

```python
team_intent = client.intent_create(
    goal="Create comprehensive report on AI trends",
    constraints={"max_turns": 20, "quality_level": "high"},
    acceptance_criteria=["All sources verified", "Report complete"]
)
```

### Agent-to-Agent Attestations

When agents communicate, each message is attested:

```python
# Researcher shares findings with analyst
client.attest_action(
    agent_id=agents["researcher"].id,
    action="message",
    target="analyst",
    intent_id=team_intent.id,
    input_data=json.dumps(findings)
)
```

### Cross-Agent Verification

You can trace the complete conversation flow:

```python
# Get all attestations linked to the team intent
all_attestations = client.attest_list(intent_id=team_intent.id)

# Verify each interaction
for att in all_attestations:
    result = client.verify_attestation(att.id)
```

## Example Output

```
=== Attest AutoGen Multi-Agent Team ===
[INFO] Creating team of 3 agents...
[INFO] Researcher: aid:a1b2c3d4
[INFO] Analyst: aid:e5f6g7h8
[INFO] Writer: aid:i9j0k1l2
[INFO] Team intent created: int:m2n3o4p5

=== Team Collaboration ===
Researcher: I'll research AI trends in healthcare...

[Researcher -> Analyst]
[attest] Attested: message -> analyst (att:123)

Analyst: I've analyzed the data. Key findings:
- 40% growth in diagnostic AI
- Regulatory challenges emerging

[Analyst -> Writer]
[attest] Attested: message -> writer (att:124)

Writer: Drafting report based on findings...

=== Session Summary ===
Team Members: 3 (researcher, analyst, writer)
Team Intent: Create comprehensive report on AI trends
Total Attestations: 47
Researcher Actions: 18
Analyst Actions: 15
Writer Actions: 14

=== Verification ===
All 47 attestations verified successfully!
Chain of custody: INTACT
```

## Project Structure

```
autogen-team/
├── README.md      # This file
├── team.py        # Multi-agent team implementation
├── requirements.txt   # Python dependencies
└── .env.example   # Environment variables template
```

## Code Overview

### Main Components

1. **`TeamAgent` class** - Wraps an AutoGen agent with Attest integration
2. **`AttestTeamManager` class** - Manages team creation and attestation
3. **`setup_team()`** - Creates all team agents with identities
4. **`run_team_discussion()`** - Executes multi-agent conversation
5. **`verify_team_actions()`** - Verifies all attestations
6. **`export_team_audit()`** - Exports complete audit trail

### Key Attest Concepts for Teams

- **Agent Identity (AID)**: Each team member has their own identity
- **Team Intent**: Shared goal that links all agents
- **Message Attestations**: Every inter-agent message is signed
- **Chain of Custody**: Complete audit trail of decisions
- **Cross-Agent Verification**: Verify any agent's actions

## Team Roles

### Researcher Agent
- Responsible for gathering information
- Creates attestations for each source verified
- Links findings to team intent

### Analyst Agent  
- Processes and synthesizes research
- Attests analytical conclusions
- Validates data quality

### Writer Agent
- Compiles final deliverables
- Attests document creation
- Tracks revision history

## Troubleshooting

### "Agent not found"

Agents may have been deleted. Recreate them:
```bash
attest agent create --name "researcher" --type "autogen"
```

### Verification fails for some attestations

Check if agents were revoked:
```bash
attest agent list --all
```

### Conversation not linking to intent

Ensure intent_id is passed to all attest_action calls:
```python
client.attest_action(
    agent_id=agent.id,
    action="message",
    target=target,
    intent_id=team_intent.id
)
```

## Next Steps

- Explore the [LangChain Chatbot example](../langchain-chatbot/) for single-agent scenarios
- Check out the [CrewAI Research example](../crewai-research/) for task automation
- Read the [Attest Documentation](https://github.com/attest/attest) for multi-agent patterns
