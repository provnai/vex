# CrewAI + Attest Research Team Example

This example demonstrates how to integrate Attest with CrewAI to create a verifiable research team where task execution, tool usage, and collaborative workflows are all cryptographically attested and auditable.

## What You'll Learn

- How to create agent identities for CrewAI agents
- How to link CrewAI tasks to Attest intents
- How to attest task execution and tool usage
- How to verify task completion against acceptance criteria
- How to export complete audit trails for research workflows

## Prerequisites

1. **Attest CLI installed** - See [Attest Installation](https://github.com/attest/attest#installation)
2. **Python 3.10+**
3. **OpenAI API key** (or another LLM provider)

## Quick Start

### 1. Install Dependencies

```bash
cd examples/crewai-research
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

# Create agent identities for the research team
attest agent create --name "senior-researcher" --type "crewai" --json
attest agent create --name "data-analyst" --type "crewai" --json
attest agent create --name "report-writer" --type "crewai" --json
```

### 4. Run the Example

```bash
python research.py
```

## How It Works

### Research Team Setup

Each CrewAI agent gets a unique Attest identity:

```python
agents = {}
for agent_config in AGENT_CONFIGS:
    agents[agent_config.role] = AttestCrewAgent(
        client=client,
        name=agent_config.name,
        role=agent_config.role,
        goal=agent_config.goal,
        backstory=agent_config.backstory
    )
```

### Task Intent Linking

Each CrewAI task is linked to a research intent:

```python
research_intent = client.intent_create(
    goal="Analyze renewable energy trends in 2024",
    constraints={
        "quality_level": "high",
        "source_verification": "required",
        "max_execution_time": "30min"
    },
    acceptance_criteria=[
        "Minimum 5 verified sources",
        "Data visualization included",
        "Executive summary provided"
    ]
)

task = Task(
    description="Research renewable energy trends...",
    expected_output="Comprehensive report with data",
    intent_id=research_intent.id
)
```

### Task Execution Attestation

Every task execution is attested:

```python
agent.attest_task_start(task, intent_id)

result = agent.execute_task(task)

agent.attest_task_complete(task, result, intent_id)
```

### Tool Usage Tracking

All tool calls during task execution are recorded:

```python
# When agent uses a search tool
client.attest_action(
    agent_id=agent.attest_id,
    action="tool_call",
    target="search",
    intent_id=intent_id,
    input_data=query
)
```

### Verification Against Criteria

After task completion, verify against acceptance criteria:

```python
verification = client.verify_against_criteria(
    intent_id=research_intent.id,
    criteria=acceptance_criteria
)
```

## Example Output

```
=== Attest CrewAI Research Team ===
[INFO] Creating research team of 3 agents...
[INFO] Senior Researcher: aid:sr12345
[INFO] Data Analyst: aid:da67890
[INFO] Report Writer: aid:rw11111

[INFO] Research intent created: int:res99999
Goal: Analyze renewable energy trends in 2024

=== Task Execution ===
Senior Researcher: Gathering data on renewable energy...
[Attest] Task started: gather_data (att:001)
[Attest] Tool call: search (att:002)
[Attest] Task complete: gather_data (att:003)

Data Analyst: Analyzing the collected data...
[Attest] Task started: analyze_data (att:004)
[Attest] Tool call: calculate (att:005)
[Attest] Task complete: analyze_data (att:006)

Report Writer: Compiling the final report...
[Attest] Task started: write_report (att:007)
[Attest] Task complete: write_report (att:008)

=== Verification ===
Acceptance Criteria:
[✓] Minimum 5 verified sources - MET (8 sources)
[✓] Data visualization included - MET (3 charts)
[✓] Executive summary provided - MET (2 pages)

=== Session Summary ===
Team: Research Team
Intent: Analyze renewable energy trends in 2024
Tasks Completed: 3
Tool Calls: 12
Total Attestations: 36

=== Verification ===
All 36 attestations verified successfully!
Chain of custody: INTACT
```

## Project Structure

```
crewai-research/
├── README.md          # This file
├── research.py        # Research team implementation
├── requirements.txt   # Python dependencies
└── .env.example       # Environment variables template
```

## Code Overview

### Main Components

1. **`AttestCrewAgent` class** - CrewAI agent with Attest integration
2. **`ResearchCrew` class** - Manages the research team workflow
3. **`TaskIntentLinker` class** - Links CrewAI tasks to intents
4. **`VerificationEngine` class** - Verifies completion against criteria
5. **`AuditExporter` class** - Exports complete audit trails

### Key Attest Concepts for Research

- **Research Intent**: Captures the research question and methodology
- **Task Attestations**: Each task execution is signed
- **Tool Call Tracking**: All data gathering is recorded
- **Source Verification**: Citations are verified and attested
- **Deliverable Signing**: Final reports are cryptographically signed

## Research Workflow

### Phase 1: Data Gathering
- Senior Researcher uses search tools
- Each search query is attested
- Sources are verified and recorded

### Phase 2: Analysis
- Data Analyst processes gathered information
- Statistical calculations are attested
- Patterns and insights are signed

### Phase 3: Reporting
- Report Writer synthesizes findings
- Document creation is attested
- Final report is signed

## Troubleshooting

### "Task not found in attestations"

Ensure the task is linked to an intent before execution:
```python
task = Task(
    description="...",
    intent_id=intent.id  # Must be set
)
```

### Verification fails for sources

Sources must be verifiable URLs or documented references:
```python
source = {
    "url": "https://...",
    "title": "...",
    "accessed_at": datetime.utcnow().isoformat()
}
client.attest_action(..., input_data=json.dumps(source))
```

### Agent identity not found

Recreate the agent identity:
```bash
attest agent create --name "senior-researcher" --type "crewai"
```

## Next Steps

- Explore the [LangChain Chatbot example](../langchain-chatbot/) for single-agent
- Check out the [AutoGen Team example](../autogen-team/) for collaborative agents
- Read the [Attest Documentation](https://github.com/attest/attest) for research patterns
