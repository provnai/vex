"""
AutoGen + Attest Multi-Agent Team Example

This module demonstrates how to integrate Attest with AutoGen to create
a verifiable multi-agent team where all agent-to-agent interactions are
cryptographically attested and auditable.

Features:
- Multiple agent identities with individual cryptographic signing
- Team-wide intent tracking across all agents
- Message attestations for inter-agent communication
- Complete chain of custody for all team decisions
- Cross-agent verification and audit trails

Usage:
    python team.py
"""

import os
import sys
import json
from datetime import datetime
from typing import Dict, List, Optional, Any
from dataclasses import dataclass, field

from dotenv import load_dotenv

sys.path.insert(0, os.path.join(os.path.dirname(__file__), "..", "..", "sdk", "python"))

from attest_client import AttestClient, AttestError, Agent, Intent, Attestation


load_dotenv()


@dataclass
class TeamAgent:
    """
    Represents an agent in the team with Attest integration.

    Each TeamAgent has:
    - A unique Attest identity for signing actions
    - A role in the team workflow
    - Statistics on actions taken
    """

    name: str
    role: str
    agent: Agent
    action_count: int = 0
    message_count: int = 0

    def to_dict(self) -> Dict[str, Any]:
        return {
            "name": self.name,
            "role": self.role,
            "agent_id": self.agent.id,
            "action_count": self.action_count,
            "message_count": self.message_count,
        }


class AttestTeamManager:
    """
    Manages a team of agents with Attest integration.

    This class handles:
    - Team agent creation and identity management
    - Team-wide intent tracking
    - Message attestation between agents
    - Team session statistics and audit trails
    """

    def __init__(self, client: AttestClient):
        """
        Initialize the team manager.

        Args:
            client: AttestClient instance for all attestation operations
        """
        self.client = client
        self.agents: Dict[str, TeamAgent] = {}
        self.team_intent: Optional[Intent] = None
        self.session_start: Optional[datetime] = None
        self.message_history: List[Dict[str, Any]] = []

    def create_team(
        self,
        team_name: str,
        agent_configs: List[Dict[str, str]],
        team_goal: str,
        constraints: Optional[Dict[str, Any]] = None,
        acceptance_criteria: Optional[List[str]] = None,
    ) -> Intent:
        """
        Create a complete team with identities and shared intent.

        Args:
            team_name: Name of the team
            agent_configs: List of agent configurations with name and role
            team_goal: The goal the team should achieve
            constraints: Team constraints (max_turns, quality_level, etc.)
            acceptance_criteria: Criteria for successful completion

        Returns:
            The created team intent
        """
        print(f"\n{'=' * 60}")
        print(f"  Creating Team: {team_name}")
        print(f"{'=' * 60}\n")

        print(f"Creating {len(agent_configs)} agent identities...")

        for config in agent_configs:
            agent = self._create_agent(config["name"], config["role"])
            self.agents[config["name"]] = TeamAgent(
                name=config["name"], role=config["role"], agent=agent
            )
            print(f"  - {config['name']} ({config['role']}): {agent.id}")

        print(f"\nCreating team intent: {team_goal[:50]}...")

        default_constraints = {
            "max_turns": 20,
            "quality_level": "high",
            "require_sources": True,
            "team_size": len(agent_configs),
        }
        constraints = {**default_constraints, **(constraints or {})}

        default_criteria = [
            "All required agents contributed",
            "Information sourced and verified",
            "Final deliverable complete",
        ]
        acceptance_criteria = acceptance_criteria or default_criteria

        self.team_intent = self.client.intent_create(
            goal=team_goal,
            constraints=constraints,
            acceptance_criteria=acceptance_criteria,
        )

        self.session_start = datetime.utcnow()

        print(f"Team intent created: {self.team_intent.id}")
        print(f"  Goal: {self.team_intent.goal}")
        print(f"  Constraints: {json.dumps(self.team_intent.constraints, indent=2)}")

        return self.team_intent

    def _create_agent(self, name: str, role: str) -> Agent:
        """
        Create a new agent identity or retrieve existing one.

        Args:
            name: Agent name
            role: Agent role in the team

        Returns:
            Created or retrieved Agent object
        """
        try:
            agents = self.client.agent_list()
            for agent in agents:
                if agent.name == name and not agent.revoked:
                    print(f"  Found existing agent: {name}")
                    return agent
        except AttestError:
            pass

        metadata = {
            "role": role,
            "team": "multi-agent",
            "created_at": datetime.utcnow().isoformat(),
        }

        agent = self.client.agent_create(
            name=name, agent_type="autogen", metadata=metadata
        )

        print(f"  Created new agent: {name} -> {agent.id}")

        return agent

    def attest_message(
        self, from_agent: str, to_agent: str, message: str, message_type: str = "info"
    ) -> Optional[Attestation]:
        """
        Attest a message sent between agents.

        This is crucial for creating an audit trail of team communication.
        Each message is signed by the sending agent and linked to the
        team intent.

        Args:
            from_agent: Name of the sending agent
            to_agent: Name of the receiving agent
            message: The message content
            message_type: Type of message (info, request, response, etc.)

        Returns:
            Created Attestation or None if it fails
        """
        if from_agent not in self.agents:
            raise ValueError(f"Unknown agent: {from_agent}")

        agent = self.agents[from_agent]

        message_record = {
            "from": from_agent,
            "to": to_agent,
            "type": message_type,
            "content": message[:500],
            "timestamp": datetime.utcnow().isoformat(),
        }

        self.message_history.append(message_record)

        agent.message_count += 1

        try:
            attestation = self.client.attest_action(
                agent_id=agent.agent.id,
                action="message",
                target=to_agent,
                intent_id=self.team_intent.id,
                input_data=json.dumps(message_record),
                session_id=f"team-{self.team_intent.id}",
            )

            print(f"  [Attest] {from_agent} -> {to_agent}: {message_type}")

            return attestation

        except AttestError as e:
            print(f"  [Attest] Failed to attest message: {e}")
            return None

    def attest_action(
        self,
        agent_name: str,
        action_type: str,
        target: str,
        input_data: Optional[str] = None,
    ) -> Optional[Attestation]:
        """
        Attest an action taken by an agent.

        Args:
            agent_name: Name of the agent taking the action
            action_type: Type of action (research, analyze, write, etc.)
            target: Target of the action
            input_data: Optional input data

        Returns:
            Created Attestation or None if it fails
        """
        if agent_name not in self.agents:
            raise ValueError(f"Unknown agent: {agent_name}")

        agent = self.agents[agent_name]
        agent.action_count += 1

        try:
            attestation = self.client.attest_action(
                agent_id=agent.agent.id,
                action=action_type,
                target=target,
                intent_id=self.team_intent.id,
                input_data=input_data,
            )

            return attestation

        except AttestError as e:
            print(f"  [Attest] Failed to attest action: {e}")
            return None

    def get_team_statistics(self) -> Dict[str, Any]:
        """Get statistics for the current team session."""
        return {
            "session_start": self.session_start.isoformat()
            if self.session_start
            else None,
            "session_duration_seconds": (
                (datetime.utcnow() - self.session_start).total_seconds()
                if self.session_start
                else 0
            ),
            "team_intent": self.team_intent.id if self.team_intent else None,
            "agents": {name: agent.to_dict() for name, agent in self.agents.items()},
            "total_messages": sum(a.message_count for a in self.agents.values()),
            "total_actions": sum(a.action_count for a in self.agents.values()),
            "message_history_count": len(self.message_history),
        }

    def verify_team_actions(self) -> Dict[str, Any]:
        """
        Verify all attestations created during the team session.

        Returns:
            Dictionary with verification results
        """
        if not self.team_intent:
            return {"error": "No team intent found"}

        print(f"\n{'=' * 60}")
        print("  Verifying Team Actions")
        print(f"{'=' * 60}\n")

        attestations = self.client.attest_list(intent_id=self.team_intent.id)

        print(f"Found {len(attestations)} attestations to verify\n")

        verified = {"total": 0, "by_agent": {}, "by_type": {}}
        failed = {"total": 0, "by_agent": {}, "by_type": {}}

        for att in attestations:
            try:
                result = self.client.verify_attestation(att.id)

                status = "verified" if result.get("valid", False) else "failed"

                verified["total"] += 1 if status == "verified" else 0
                failed["total"] += 1 if status == "failed" else 0

                agent_name = att.agent_name
                action_type = att.action_type

                if agent_name not in verified["by_agent"]:
                    verified["by_agent"][agent_name] = 0
                    failed["by_agent"][agent_name] = 0

                if action_type not in verified["by_type"]:
                    verified["by_type"][action_type] = 0
                    failed["by_type"][action_type] = 0

                if status == "verified":
                    verified["by_agent"][agent_name] += 1
                    verified["by_type"][action_type] += 1
                else:
                    failed["by_agent"][agent_name] += 1
                    failed["by_type"][action_type] += 1

            except Exception as e:
                failed["total"] += 1
                print(f"  [✗] {att.id}: {e}")

        print("\nVerification Summary:")
        print(f"  Total Attestations: {len(attestations)}")
        print(f"  Verified: {verified['total']}")
        print(f"  Failed: {failed['total']}")

        print("\nBy Agent:")
        for name in sorted(verified["by_agent"].keys()):
            v = verified["by_agent"].get(name, 0)
            f = failed["by_agent"].get(name, 0)
            print(f"  {name}: {v} verified, {f} failed")

        print("\nBy Type:")
        for atype in sorted(verified["by_type"].keys()):
            v = verified["by_type"].get(atype, 0)
            f = failed["by_type"].get(atype, 0)
            print(f"  {atype}: {v} verified, {f} failed")

        return {
            "total": len(attestations),
            "verified": verified,
            "failed": failed,
            "success_rate": verified["total"] / len(attestations)
            if attestations
            else 0,
        }

    def export_team_audit(self) -> Dict[str, Any]:
        """
        Export complete audit trail for the team session.

        Returns:
            Complete audit data including all attestations
        """
        if not self.team_intent:
            return {"error": "No team intent found"}

        attestations = self.client.attest_list(intent_id=self.team_intent.id)

        audit_data = {
            "exported_at": datetime.utcnow().isoformat(),
            "version": "1.0",
            "team_intent": {
                "id": self.team_intent.id,
                "goal": self.team_intent.goal,
                "constraints": self.team_intent.constraints,
                "acceptance_criteria": self.team_intent.acceptance_criteria,
                "status": self.team_intent.status,
                "created_at": self.team_intent.created_at,
            },
            "agents": {
                name: {
                    "id": agent.agent.id,
                    "name": agent.name,
                    "role": agent.role,
                    "type": agent.agent.type,
                    "public_key": agent.agent.public_key,
                    "action_count": agent.action_count,
                    "message_count": agent.message_count,
                }
                for name, agent in self.agents.items()
            },
            "statistics": self.get_team_statistics(),
            "attestations": {
                "total": len(attestations),
                "records": [
                    {
                        "id": att.id,
                        "agent_id": att.agent_id,
                        "agent_name": att.agent_name,
                        "action_type": att.action_type,
                        "target": att.action_target,
                        "input_summary": att.action_input[:100]
                        if att.action_input
                        else None,
                        "timestamp": att.timestamp,
                        "signature": att.signature[:50] + "..."
                        if att.signature
                        else None,
                    }
                    for att in attestations
                ],
            },
            "message_history": self.message_history,
        }

        export_path = os.path.join(os.path.dirname(__file__), "team_audit_export.json")

        with open(export_path, "w") as f:
            json.dump(audit_data, f, indent=2, default=str)

        print(f"\nTeam audit exported to: {export_path}")

        return audit_data


def print_section(title: str) -> None:
    """Print a formatted section header."""
    print(f"\n{'=' * 60}")
    print(f"  {title}")
    print(f"{'=' * 60}\n")


def simulate_team_discussion(manager: AttestTeamManager) -> None:
    """
    Simulate a multi-agent team discussion with full attestation.

    This demonstrates:
    1. Research agent gathering information
    2. Analyst agent processing findings
    3. Writer agent creating deliverable
    4. All messages attested between agents
    """
    print_section("Team Collaboration")

    research_topic = "AI in Healthcare 2024"

    researcher = manager.agents["researcher"]
    analyst = manager.agents["analyst"]
    writer = manager.agents["writer"]

    print(f"Topic: {research_topic}\n")

    print(f"{researcher.name}: I'll start researching {research_topic}...")
    manager.attest_action(
        researcher.name,
        "research_start",
        research_topic,
        json.dumps({"topic": research_topic, "depth": "comprehensive"}),
    )

    findings = {
        "market_size": "$150B by 2030",
        "key_applications": ["diagnostics", "drug discovery", "personalized medicine"],
        "growth_rate": "37% annually",
        "challenges": ["data privacy", "regulation", "bias in AI"],
    }

    print(f"\n{researcher.name}: Research complete. Key findings:")
    for key, value in findings.items():
        print(f"  - {key}: {value}")

    manager.attest_message(
        researcher.name,
        analyst.name,
        f"Research findings for {research_topic}: {json.dumps(findings)}",
        "findings",
    )

    manager.attest_action(
        researcher.name, "research_complete", research_topic, json.dumps(findings)
    )

    print(f"\n{analyst.name}: Analyzing the research data...")
    manager.attest_action(analyst.name, "analysis_start", research_topic)

    analysis = {
        "market_opportunity": "High - growing 37% annually",
        "risk_factors": ["Regulatory uncertainty", "Data privacy concerns"],
        "recommendation": "Invest in diagnostic AI with focus on regulatory compliance",
        "timeline": "18-24 months to market",
    }

    print(f"\n{analyst.name}: Analysis complete:")
    for key, value in analysis.items():
        print(f"  - {key}: {value}")

    manager.attest_message(
        analyst.name,
        writer.name,
        f"Analysis complete: {json.dumps(analysis)}",
        "analysis",
    )

    manager.attest_action(
        analyst.name, "analysis_complete", research_topic, json.dumps(analysis)
    )

    print(f"\n{writer.name}: Drafting the report...")
    manager.attest_action(writer.name, "draft_start", research_topic)

    report_outline = {
        "title": f"{research_topic} Market Analysis",
        "sections": [
            "Executive Summary",
            "Market Overview",
            "Key Findings",
            "Recommendations",
        ],
        "pages": 12,
        "appendices": ["Data Sources", "Methodology"],
    }

    print(f"\n{writer.name}: Report outline complete:")
    print(f"  Title: {report_outline['title']}")
    print(f"  Sections: {', '.join(report_outline['sections'])}")

    manager.attest_message(
        writer.name,
        "all",
        f"Report drafted: {json.dumps(report_outline)}",
        "deliverable",
    )

    manager.attest_action(
        writer.name, "draft_complete", research_topic, json.dumps(report_outline)
    )

    print(f"\n{writer.name}: Report complete and ready for review!")


def main() -> None:
    """Main entry point for the multi-agent team demo."""
    print_section("Attest AutoGen Multi-Agent Team Demo")
    print("This demo shows how Attest provides verifiable multi-agent collaboration.\n")

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
        manager = AttestTeamManager(client)

        agent_configs = [
            {"name": "researcher", "role": "research"},
            {"name": "analyst", "role": "analysis"},
            {"name": "writer", "role": "writing"},
        ]

        team_goal = (
            "Create a comprehensive market analysis report on AI in Healthcare 2024"
        )

        manager.create_team(
            team_name="AI Research Team",
            agent_configs=agent_configs,
            team_goal=team_goal,
            constraints={"max_turns": 15, "quality_level": "high"},
            acceptance_criteria=[
                "Market data verified from multiple sources",
                "Analysis includes risk assessment",
                "Report includes executive summary and recommendations",
            ],
        )

        simulate_team_discussion(manager)

        manager.verify_team_actions()

        audit = manager.export_team_audit()

        print_section("Demo Complete")
        print("What we demonstrated:")
        print("  1. Team agent creation - each agent has unique identity")
        print("  2. Team intent - shared goal linking all agents")
        print("  3. Message attestations - every inter-agent message signed")
        print("  4. Action attestations - all agent actions recorded")
        print("  5. Cross-agent verification - verify any agent's actions")
        print("  6. Complete audit trail - export for compliance")

        print("\nStatistics:")
        stats = manager.get_team_statistics()
        print(f"  Team Size: {len(stats['agents'])} agents")
        print(f"  Total Messages: {stats['total_messages']}")
        print(f"  Total Actions: {stats['total_actions']}")

        print("\nNext steps:")
        print("  - Explore the LangChain chatbot example for single-agent")
        print("  - Check the CrewAI research example for task automation")
        print("  - Read full Attest docs for multi-agent patterns")

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
