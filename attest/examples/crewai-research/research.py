"""
CrewAI + Attest Research Team Example

This module demonstrates how to integrate Attest with CrewAI to create
a verifiable research team where all task execution, tool usage, and
collaborative workflows are cryptographically attested.

Features:
- Agent identities for each research team member
- Task-to-intent linking for research goals
- Tool call attestation during research activities
- Verification of task completion against criteria
- Complete audit trail export for research workflows

Usage:
    python research.py
"""

import os
import sys
import json
from datetime import datetime
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field
from enum import Enum

from dotenv import load_dotenv

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "sdk", "python"))

from attest_client import AttestClient, AttestError, Agent, Intent, Attestation


load_dotenv()


class TaskStatus(Enum):
    """Status of a research task."""

    PENDING = "pending"
    IN_PROGRESS = "in_progress"
    COMPLETED = "completed"
    FAILED = "failed"
    VERIFIED = "verified"


@dataclass
class ResearchTask:
    """
    Represents a research task with Attest integration.

    Each task has:
    - Unique identifier
    - Description and expected output
    - Linked Attest intent
    - Execution status and results
    """

    id: str
    description: str
    expected_output: str
    agent_role: str
    intent_id: Optional[str] = None
    status: TaskStatus = TaskStatus.PENDING
    result: Optional[str] = None
    sources: List[Dict[str, str]] = field(default_factory=list)
    tools_used: List[Dict[str, Any]] = field(default_factory=list)
    attestations: List[str] = field(default_factory=list)
    started_at: Optional[datetime] = None
    completed_at: Optional[datetime] = None

    def to_dict(self) -> Dict[str, Any]:
        return {
            "id": self.id,
            "description": self.description,
            "expected_output": self.expected_output,
            "agent_role": self.agent_role,
            "intent_id": self.intent_id,
            "status": self.status.value,
            "result": self.result,
            "sources": self.sources,
            "tools_used": self.tools_used,
            "attestation_count": len(self.attestations),
            "started_at": self.started_at.isoformat() if self.started_at else None,
            "completed_at": self.completed_at.isoformat()
            if self.completed_at
            else None,
        }


@dataclass
class AgentConfig:
    """Configuration for a research team agent."""

    name: str
    role: str
    goal: str
    backstory: str


AGENT_CONFIGS = [
    AgentConfig(
        name="Dr. Sarah Chen",
        role="senior_researcher",
        goal="Gather comprehensive data on research topics",
        backstory="Expert researcher with 15 years of experience in data collection and source verification.",
    ),
    AgentConfig(
        name="Marcus Johnson",
        role="data_analyst",
        goal="Analyze data and identify patterns",
        backstory="Statistical analyst skilled in quantitative methods and data visualization.",
    ),
    AgentConfig(
        name="Emily Rodriguez",
        role="report_writer",
        goal="Synthesize findings into comprehensive reports",
        backstory="Technical writer who transforms complex data into clear, actionable insights.",
    ),
]


class AttestCrewAgent:
    """
    CrewAI agent with Attest integration for verifiable research.

    Each agent:
    - Has a unique Attest identity for signing actions
    - Records all task executions with cryptographic signatures
    - Tracks tool usage for audit purposes
    - Links all activities to research intents
    """

    def __init__(self, client: AttestClient, config: AgentConfig, agent: Agent):
        """
        Initialize the agent with Attest integration.

        Args:
            client: AttestClient for attestation operations
            config: Agent configuration
            agent: Attest agent identity
        """
        self.client = client
        self.config = config
        self.agent = agent
        self.task_count = 0
        self.tool_call_count = 0
        self.current_tasks: List[ResearchTask] = []

    @property
    def attest_id(self) -> str:
        """Get the agent's Attest ID."""
        return self.agent.id

    def attest_task_start(
        self, task: ResearchTask, intent_id: str
    ) -> Optional[Attestation]:
        """
        Attest the start of a task.

        Args:
            task: The task being started
            intent_id: The research intent this task supports

        Returns:
            Attestation record or None
        """
        task.status = TaskStatus.IN_PROGRESS
        task.started_at = datetime.utcnow()
        task.intent_id = intent_id

        try:
            attestation = self.client.attest_action(
                agent_id=self.agent.id,
                action="task_start",
                target=task.id,
                intent_id=intent_id,
                input_data=json.dumps(
                    {
                        "description": task.description,
                        "expected_output": task.expected_output,
                        "agent_role": task.agent_role,
                    }
                ),
            )

            task.attestations.append(attestation.id)
            self.task_count += 1

            print(f"  [Attest] Task started: {task.id}")

            return attestation

        except AttestError as e:
            print(f"  [Attest] Warning: Failed to attest task start: {e}")
            return None

    def attest_tool_call(
        self,
        tool_name: str,
        input_data: str,
        output: Optional[str] = None,
        intent_id: Optional[str] = None,
    ) -> Optional[Attestation]:
        """
        Attest a tool call during task execution.

        Args:
            tool_name: Name of the tool being used
            input_data: Input provided to the tool
            output: Output from the tool
            intent_id: Optional intent ID

        Returns:
            Attestation record or None
        """
        tool_record = {
            "tool": tool_name,
            "input": input_data[:500],
            "output": output[:500] if output else None,
            "timestamp": datetime.utcnow().isoformat(),
        }

        self.tool_call_count += 1

        try:
            attestation = self.client.attest_action(
                agent_id=self.agent.id,
                action="tool_call",
                target=tool_name,
                intent_id=intent_id or self.current_tasks[-1].intent_id
                if self.current_tasks
                else None,
                input_data=json.dumps(tool_record),
            )

            if self.current_tasks:
                self.current_tasks[-1].tools_used.append(tool_record)

            return attestation

        except AttestError as e:
            print(f"  [Attest] Warning: Failed to attest tool call: {e}")
            return None

    def attest_task_complete(
        self,
        task: ResearchTask,
        result: str,
        intent_id: str,
        sources: Optional[List[Dict[str, str]]] = None,
    ) -> Optional[Attestation]:
        """
        Attest the completion of a task.

        Args:
            task: The completed task
            result: The task result/output
            intent_id: The research intent
            sources: List of sources used in the task

        Returns:
            Attestation record or None
        """
        task.status = TaskStatus.COMPLETED
        task.completed_at = datetime.utcnow()
        task.result = result
        if sources:
            task.sources = sources

        try:
            completion_data = {
                "task_id": task.id,
                "result_summary": result[:500],
                "sources_used": len(sources) if sources else 0,
                "tools_used": len(task.tools_used),
                "duration_seconds": (
                    task.completed_at - task.started_at
                ).total_seconds(),
            }

            attestation = self.client.attest_action(
                agent_id=self.agent.id,
                action="task_complete",
                target=task.id,
                intent_id=intent_id,
                input_data=json.dumps(completion_data),
            )

            task.attestations.append(attestation.id)

            print(f"  [Attest] Task complete: {task.id}")

            return attestation

        except AttestError as e:
            print(f"  [Attest] Warning: Failed to attest task completion: {e}")
            return None

    def attest_source_verification(
        self, source: Dict[str, str], intent_id: str
    ) -> Optional[Attestation]:
        """
        Attest verification of a research source.

        Args:
            source: Source information (url, title, etc.)
            intent_id: Research intent

        Returns:
            Attestation record or None
        """
        try:
            attestation = self.client.attest_action(
                agent_id=self.agent.id,
                action="source_verification",
                target=source.get("url", "unknown"),
                intent_id=intent_id,
                input_data=json.dumps(source),
            )

            print(
                f"  [Attest] Source verified: {source.get('title', 'Unknown')[:40]}..."
            )

            return attestation

        except AttestError as e:
            print(f"  [Attest] Warning: Failed to attest source: {e}")
            return None


class ResearchCrew:
    """
    Manages a CrewAI research team with Attest integration.

    This class handles:
    - Team agent creation and management
    - Research intent creation and tracking
    - Task execution orchestration
    - Verification against acceptance criteria
    - Audit trail export
    """

    def __init__(self, client: AttestClient):
        """
        Initialize the research crew.

        Args:
            client: AttestClient for all operations
        """
        self.client = client
        self.agents: Dict[str, AttestCrewAgent] = {}
        self.research_intent: Optional[Intent] = None
        self.tasks: List[ResearchTask] = []
        self.sources_verified: List[Dict[str, str]] = []

    def setup_team(self) -> Dict[str, AttestCrewAgent]:
        """
        Set up the research team with Attest identities.

        Returns:
            Dictionary of agents by role
        """
        print(f"\n{'=' * 60}")
        print(f"  Setting Up Research Team")
        print(f"{'=' * 60}\n")

        for config in AGENT_CONFIGS:
            agent = self._get_or_create_agent(config)

            self.agents[config.role] = AttestCrewAgent(
                client=self.client, config=config, agent=agent
            )

            print(f"  - {config.name} ({config.role}): {agent.id}")

        return self.agents

    def _get_or_create_agent(self, config: AgentConfig) -> Agent:
        """Get existing agent or create new one."""
        try:
            agents = self.client.agent_list()
            for agent in agents:
                if agent.name == config.name and not agent.revoked:
                    print(f"  Found existing agent: {config.name}")
                    return agent
        except AttestError:
            pass

        metadata = {
            "role": config.role,
            "team": "research",
            "goal": config.goal,
            "created_at": datetime.utcnow().isoformat(),
        }

        agent = self.client.agent_create(
            name=config.name, agent_type="crewai", metadata=metadata
        )

        print(f"  Created new agent: {config.name} -> {agent.id}")

        return agent

    def create_research_intent(
        self,
        research_topic: str,
        constraints: Optional[Dict[str, Any]] = None,
        acceptance_criteria: Optional[List[str]] = None,
    ) -> Intent:
        """
        Create a research intent for the team.

        Args:
            research_topic: The research question or topic
            constraints: Research constraints
            acceptance_criteria: Criteria for successful completion

        Returns:
            Created Intent
        """
        print(f"\n{'=' * 60}")
        print(f"  Creating Research Intent")
        print(f"{'=' * 60}\n")

        default_constraints = {
            "quality_level": "high",
            "source_verification": "required",
            "min_sources": 5,
            "max_execution_time": "30min",
            "data_visualization": "required",
        }
        constraints = {**(constraints or {}), **default_constraints}

        default_criteria = [
            "Minimum 5 verified sources",
            "Data visualization included",
            "Executive summary provided",
            "Methodology documented",
            "Conclusions supported by data",
        ]
        acceptance_criteria = acceptance_criteria or default_criteria

        self.research_intent = self.client.intent_create(
            goal=research_topic,
            constraints=constraints,
            acceptance_criteria=acceptance_criteria,
        )

        print(f"Research intent created: {self.research_intent.id}")
        print(f"  Goal: {self.research_intent.goal}")
        print(
            f"  Constraints: {json.dumps(self.research_intent.constraints, indent=2)}"
        )
        print(f"  Criteria: {len(self.research_intent.acceptance_criteria)} items")

        return self.research_intent

    def execute_research_task(self, task: ResearchTask) -> ResearchTask:
        """
        Execute a research task with full attestation.

        Args:
            task: The task to execute

        Returns:
            Updated task with results
        """
        agent = self.agents.get(task.agent_role)

        if not agent:
            raise ValueError(f"Unknown agent role: {task.agent_role}")

        print(f"\n{agent.config.name}: {task.description}")

        agent.attest_task_start(task, self.research_intent.id)

        self.tasks.append(task)

        if task.agent_role == "senior_researcher":
            result, sources = self._execute_research_task(task, agent)
        elif task.agent_role == "data_analyst":
            result, sources = self._execute_analysis_task(task, agent)
        else:
            result, sources = self._execute_writing_task(task, agent)

        agent.attest_task_complete(task, result, self.research_intent.id, sources)

        if sources:
            self.sources_verified.extend(sources)

        return task

    def _execute_research_task(
        self, task: ResearchTask, agent: AttestCrewAgent
    ) -> tuple[str, List[Dict[str, str]]]:
        """Execute data gathering task."""
        sources = [
            {
                "url": "https://example.com/source1",
                "title": "Renewable Energy Statistics 2024",
                "accessed_at": datetime.utcnow().isoformat(),
                "reliability": "high",
            },
            {
                "url": "https://example.com/source2",
                "title": "Global Energy Market Analysis",
                "accessed_at": datetime.utcnow().isoformat(),
                "reliability": "high",
            },
        ]

        for source in sources:
            agent.attest_source_verification(source, self.research_intent.id)

        simulated_tools = [
            {"tool": "web_search", "query": "renewable energy statistics 2024"},
            {"tool": "database_query", "query": "energy production data"},
            {"tool": "document_analysis", "query": "market reports"},
        ]

        for tool in simulated_tools:
            agent.attest_tool_call(
                tool["tool"],
                tool["query"],
                f"Results for: {tool['query']}",
                self.research_intent.id,
            )

        result = f"""Research completed on renewable energy trends:

Key Findings:
1. Solar capacity grew 40% year-over-year
2. Wind energy accounts for 25% of new installations
3. Battery storage costs decreased 30%

Sources consulted: {len(sources)}
Data coverage: Global markets"""

        return result, sources

    def _execute_analysis_task(
        self, task: ResearchTask, agent: AttestCrewAgent
    ) -> tuple[str, List[Dict[str, str]]]:
        """Execute data analysis task."""
        analysis_tools = [
            {"tool": "statistical_analysis", "input": "growth rates"},
            {"tool": "trend_analysis", "input": "5-year projections"},
            {"tool": "correlation_analysis", "input": "factors affecting adoption"},
        ]

        for tool in analysis_tools:
            agent.attest_tool_call(
                tool["tool"],
                tool["input"],
                f"Analysis results for: {tool['input']}",
                self.research_intent.id,
            )

        sources = [
            {
                "url": "https://example.com/analysis1",
                "title": "Statistical Analysis Report",
                "accessed_at": datetime.utcnow().isoformat(),
                "reliability": "high",
            }
        ]

        result = """Analysis completed with following insights:

Statistical Summary:
- Average growth rate: 35% (2020-2024)
- Projected 2028 market size: $500B
- Key drivers: Policy support, cost reduction

Visualizations generated:
1. Growth trend chart
2. Market share pie chart
3. Regional comparison map"""

        return result, sources

    def _execute_writing_task(
        self, task: ResearchTask, agent: AttestCrewAgent
    ) -> tuple[str, List[Dict[str, str]]]:
        """Execute report writing task."""
        agent.attest_tool_call(
            "document_generation",
            task.expected_output,
            "Draft created",
            self.research_intent.id,
        )

        result = """Report: Renewable Energy Trends 2024

EXECUTIVE SUMMARY
-----------------
The renewable energy sector has demonstrated exceptional growth in 2024,
with solar and wind leading the transition to clean energy.

KEY INSIGHTS
------------
1. Solar photovoltaic capacity increased by 40%
2. Offshore wind installations reached record levels
3. Energy storage costs declined significantly

MARKET PROJECTIONS
------------------
By 2028, the renewable energy market is projected to reach $500B,
representing a compound annual growth rate of 35%.

RECOMMENDATIONS
---------------
- Increase investment in solar infrastructure
- Support offshore wind development
- Develop grid integration solutions

CONCLUSION
----------
The renewable energy sector presents significant opportunities for
investment and innovation, driven by technological advances and
supportive policy environments."""

        return result, []

    def verify_completion(self) -> Dict[str, Any]:
        """
        Verify task completion against acceptance criteria.

        Returns:
            Verification results
        """
        print(f"\n{'=' * 60}")
        print(f"  Verifying Task Completion")
        print(f"{'=' * 60}\n")

        if not self.research_intent:
            return {"error": "No research intent found"}

        criteria = self.research_intent.acceptance_criteria

        print("Acceptance Criteria Verification:\n")

        verification_results = []

        for criterion in criteria:
            if "sources" in criterion.lower():
                source_count = len(self.sources_verified)
                min_sources = self.research_intent.constraints.get("min_sources", 5)
                met = source_count >= min_sources
                status = "✓" if met else "✗"
                detail = f"{source_count} sources (minimum: {min_sources})"

            elif "visualization" in criterion.lower():
                viz_count = sum(
                    1
                    for task in self.tasks
                    if task.result and "chart" in task.result.lower()
                )
                met = viz_count >= 1
                status = "✓" if met else "✗"
                detail = f"{viz_count} visualizations found"

            elif "executive summary" in criterion.lower():
                has_summary = any(
                    task.result and "executive summary" in task.result.lower()
                    for task in self.tasks
                )
                met = has_summary
                status = "✓" if met else "✗"
                detail = "Executive summary included" if met else "Missing"

            else:
                met = True
                status = "✓"
                detail = "Assumed met"

            verification_results.append(
                {"criterion": criterion, "met": met, "detail": detail}
            )

            print(f"  [{status}] {criterion}")
            print(f"       {detail}\n")

        all_met = all(r["met"] for r in verification_results)

        print(
            f"\nOverall Status: {'ALL CRITERIA MET' if all_met else 'SOME CRITERIA NOT MET'}"
        )

        return {
            "all_met": all_met,
            "criteria": verification_results,
            "sources_verified": len(self.sources_verified),
            "tasks_completed": len(self.tasks),
        }

    def verify_all_attestations(self) -> Dict[str, Any]:
        """Verify all attestations created during research."""
        print(f"\n{'=' * 60}")
        print(f"  Verifying All Attestations")
        print(f"{'=' * 60}\n")

        if not self.research_intent:
            return {"error": "No research intent found"}

        attestations = self.client.attest_list(intent_id=self.research_intent.id)

        print(f"Found {len(attestations)} attestations to verify\n")

        verified = 0
        failed = 0

        for att in attestations:
            try:
                result = self.client.verify_attestation(att.id)
                if result.get("valid", False):
                    print(f"  [✓] {att.action_type}: {att.action_target[:40]}...")
                    verified += 1
                else:
                    print(
                        f"  [✗] {att.action_type}: {att.action_target[:40]}... FAILED"
                    )
                    failed += 1
            except Exception as e:
                print(
                    f"  [✗] {att.action_type}: {att.action_target[:40]}... ERROR: {e}"
                )
                failed += 1

        print(f"\nVerification: {verified} verified, {failed} failed")

        return {"total": len(attestations), "verified": verified, "failed": failed}

    def export_research_audit(self) -> Dict[str, Any]:
        """
        Export complete research audit trail.

        Returns:
            Complete audit data
        """
        if not self.research_intent:
            return {"error": "No research intent found"}

        attestations = self.client.attest_list(intent_id=self.research_intent.id)

        audit_data = {
            "exported_at": datetime.utcnow().isoformat(),
            "version": "1.0",
            "research_intent": {
                "id": self.research_intent.id,
                "goal": self.research_intent.goal,
                "constraints": self.research_intent.constraints,
                "acceptance_criteria": self.research_intent.acceptance_criteria,
                "status": self.research_intent.status,
                "created_at": self.research_intent.created_at,
            },
            "team": {
                role: {
                    "name": agent.config.name,
                    "attest_id": agent.agent.id,
                    "tasks_completed": agent.task_count,
                    "tool_calls": agent.tool_call_count,
                }
                for role, agent in self.agents.items()
            },
            "tasks": {
                "total": len(self.tasks),
                "completed": sum(
                    1 for t in self.tasks if t.status == TaskStatus.COMPLETED
                ),
                "records": [t.to_dict() for t in self.tasks],
            },
            "sources": {
                "total": len(self.sources_verified),
                "records": self.sources_verified,
            },
            "attestations": {"total": len(attestations), "by_type": {}},
            "verification": self.verify_completion(),
        }

        for att in attestations:
            atype = att.action_type
            if atype not in audit_data["attestations"]["by_type"]:
                audit_data["attestations"]["by_type"][atype] = 0
            audit_data["attestations"]["by_type"][atype] += 1

        export_path = os.path.join(
            os.path.dirname(__file__), "research_audit_export.json"
        )

        with open(export_path, "w") as f:
            json.dump(audit_data, f, indent=2, default=str)

        print(f"\nResearch audit exported to: {export_path}")

        return audit_data


def print_section(title: str) -> None:
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def main() -> None:
    """Main entry point for the research team demo."""
    print_section("Attest CrewAI Research Team Demo")
    print("This demo shows how Attest provides verifiable research workflows.\n")

    try:
        client = AttestClient(verbose=True)
        print("[INFO] Attest client initialized")

    except AttestError as e:
        print(f"[ERROR] Failed to initialize Attest client: {e}")
        print("\nPlease ensure Attest is installed and initialized:")
        print("  1. Install Attest: go install github.com/attest/attest")
        print("  2. Initialize: attest init")
        sys.exit(1)

    if not os.getenv("OPENAI_API_KEY"):
        print("[ERROR] OPENAI_API_KEY not set")
        print("Please create a .env file with your API key")
        sys.exit(1)

    try:
        crew = ResearchCrew(client)

        crew.setup_team()

        crew.create_research_intent(
            research_topic="Analyze renewable energy market trends and projections for 2024"
        )

        tasks = [
            ResearchTask(
                id="task-001",
                description="Gather comprehensive data on renewable energy trends",
                expected_output="Dataset with statistics on solar, wind, and storage",
                agent_role="senior_researcher",
            ),
            ResearchTask(
                id="task-002",
                description="Analyze collected data for patterns and projections",
                expected_output="Statistical analysis with visualizations",
                agent_role="data_analyst",
            ),
            ResearchTask(
                id="task-003",
                description="Compile comprehensive research report",
                expected_output="Full research report with executive summary",
                agent_role="report_writer",
            ),
        ]

        for task in tasks:
            crew.execute_research_task(task)

        crew.verify_completion()

        crew.verify_all_attestations()

        audit = crew.export_research_audit()

        print_section("Demo Complete")
        print("What we demonstrated:")
        print("  1. Research team setup with agent identities")
        print("  2. Research intent creation with goals and criteria")
        print("  3. Task execution with full attestation")
        print("  4. Tool usage tracking during research")
        print("  5. Source verification and recording")
        print("  6. Verification against acceptance criteria")
        print("  7. Complete audit trail export")

        print("\nStatistics:")
        print(f"  Team Size: {len(crew.agents)} agents")
        print(f"  Tasks Completed: {len(crew.tasks)}")
        print(f"  Sources Verified: {len(crew.sources_verified)}")
        print(
            f"  Total Attestations: {len(audit.get('attestations', {}).get('total', 0))}"
        )

        print("\nNext steps:")
        print("  - Explore LangChain chatbot for single-agent scenarios")
        print("  - Check AutoGen team for collaborative agents")
        print("  - Read full Attest docs for advanced patterns")

    except KeyboardInterrupt:
        print("\n\nDemo interrupted by user")
        sys.exit(0)
    except Exception as e:
        print(f"\n[ERROR] Demo failed: {e}")
        import traceback

        traceback.print_exc()
        sys.exit(1)


if __name__ == "__main__":
    main()
