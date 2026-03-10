package integrations

import (
	"os"
	"path/filepath"
)

func GetCrewAITemplate() string {
	return `"""
Attest Integration for CrewAI
Import this module to enable Attest monitoring for CrewAI crews.

Usage:
    from attest.attest_crew_setup import setup_attest_for_crew
    from crewai import Crew, Agent, Task
    
    crew = Crew(...)
    setup_attest_for_crew(crew)
    
    # Run your crew with monitoring
    result = crew.kickoff()
"""

from typing import Any, Dict, List, Optional
import json
import time
import requests
from functools import wraps


class AttestCrewMonitor:
    """Monitor for CrewAI crews, agents, and tasks."""
    
    def __init__(
        self, 
        api_key: Optional[str] = None, 
        endpoint: Optional[str] = None,
        crew_name: Optional[str] = None
    ):
        self.api_key = api_key or self._get_api_key()
        self.endpoint = endpoint or "https://api.attest.dev/v1"
        self.session_id = self._generate_session_id()
        self.crew_name = crew_name or "unnamed_crew"
        self.task_results = []
        
    def _get_api_key(self) -> str:
        """Get API key from environment."""
        import os
        return os.getenv("ATTEST_API_KEY", "")
    
    def _generate_session_id(self) -> str:
        """Generate unique session ID."""
        import uuid
        return str(uuid.uuid4())
    
    def _send_event(self, event_type: str, data: Dict[str, Any]):
        """Send event to Attest API."""
        if not self.api_key:
            return
            
        payload = {
            "session_id": self.session_id,
            "crew_name": self.crew_name,
            "event_type": event_type,
            "timestamp": time.time(),
            "data": data
        }
        
        try:
            headers = {"Authorization": f"Bearer {self.api_key}"}
            requests.post(
                f"{self.endpoint}/events",
                json=payload,
                headers=headers,
                timeout=5
            )
        except:
            pass
    
    def monitor_crew_start(self, agents: List[Any], tasks: List[Any]):
        """Monitor when crew starts execution."""
        agent_data = []
        for agent in agents:
            agent_data.append({
                "name": getattr(agent, 'role', 'unknown'),
                "goal": getattr(agent, 'goal', '')[:200],
                "backstory": getattr(agent, 'backstory', '')[:200],
                "tools": [str(t) for t in getattr(agent, 'tools', [])]
            })
        
        task_data = []
        for task in tasks:
            task_data.append({
                "description": getattr(task, 'description', '')[:200],
                "expected_output": getattr(task, 'expected_output', '')[:200],
                "agent": getattr(getattr(task, 'agent', None), 'role', 'unassigned')
            })
        
        self._send_event("crew_start", {
            "crew_name": self.crew_name,
            "agent_count": len(agents),
            "task_count": len(tasks),
            "agents": agent_data,
            "tasks": task_data
        })
    
    def monitor_task_start(self, task: Any, agent: Any):
        """Monitor task execution start."""
        self._send_event("task_start", {
            "task_description": getattr(task, 'description', 'unknown')[:200],
            "agent": getattr(agent, 'role', 'unknown'),
            "expected_output": getattr(task, 'expected_output', '')[:200]
        })
    
    def monitor_task_complete(
        self, 
        task: Any, 
        agent: Any, 
        result: Any,
        duration: float
    ):
        """Monitor task completion."""
        result_str = str(result)
        self.task_results.append({
            "task": getattr(task, 'description', 'unknown')[:100],
            "agent": getattr(agent, 'role', 'unknown'),
            "duration": duration,
            "output_preview": result_str[:500]
        })
        
        self._send_event("task_complete", {
            "task_description": getattr(task, 'description', 'unknown')[:200],
            "agent": getattr(agent, 'role', 'unknown'),
            "duration": duration,
            "result_preview": result_str[:1000],
            "result_length": len(result_str)
        })
    
    def monitor_crew_complete(self, final_output: Any, total_duration: float):
        """Monitor crew execution completion."""
        output_str = str(final_output)
        
        self._send_event("crew_complete", {
            "total_duration": total_duration,
            "output_preview": output_str[:2000],
            "output_length": len(output_str),
            "tasks_completed": len(self.task_results),
            "task_summary": self.task_results
        })
    
    def monitor_tool_usage(
        self, 
        tool_name: str, 
        inputs: Dict[str, Any],
        output: Any,
        duration: Optional[float] = None
    ):
        """Monitor tool usage by agents."""
        self._send_event("tool_usage", {
            "tool": tool_name,
            "inputs": {k: str(v)[:200] for k, v in inputs.items()},
            "output_preview": str(output)[:500],
            "duration": duration
        })


def setup_attest_for_crew(crew: Any, **kwargs) -> AttestCrewMonitor:
    """
    Setup Attest monitoring for a CrewAI crew.
    """
    crew_name = kwargs.get('crew_name', getattr(crew, 'name', 'unnamed_crew'))
    monitor = AttestCrewMonitor(crew_name=crew_name, **kwargs)
    
    if hasattr(crew, 'kickoff'):
        original_kickoff = crew.kickoff
        
        @wraps(original_kickoff)
        def monitored_kickoff(*args, **kickoff_kwargs):
            start_time = time.time()

            agents = getattr(crew, 'agents', [])
            tasks = getattr(crew, 'tasks', [])

            monitor.monitor_crew_start(agents, tasks)
            
            try:
                result = original_kickoff(*args, **kickoff_kwargs)
                duration = time.time() - start_time
                monitor.monitor_crew_complete(result, duration)
                return result
            except Exception as e:
                monitor._send_event("crew_error", {
                    "error": str(e),
                    "error_type": type(e).__name__,
                    "duration": time.time() - start_time
                })
                raise
        
        crew.kickoff = monitored_kickoff
    
    for agent in getattr(crew, 'agents', []):
        _wrap_agent(agent, monitor)
    
    return monitor


def _wrap_agent(agent: Any, monitor: AttestCrewMonitor):
    """Wrap an agent's execute_task method."""
    if hasattr(agent, 'execute_task'):
        original_execute = agent.execute_task
        
        @wraps(original_execute)
        def monitored_execute(task: Any, *args, **kwargs):
            start_time = time.time()
            monitor.monitor_task_start(task, agent)
            result = original_execute(task, *args, **kwargs)
            duration = time.time() - start_time
            monitor.monitor_task_complete(task, agent, result, duration)
            return result
        
        agent.execute_task = monitored_execute


def create_monitored_crew(
    agents: List[Any],
    tasks: List[Any],
    monitor: Optional[AttestCrewMonitor] = None,
    **crew_kwargs
):
    """
    Create a CrewAI crew with Attest monitoring pre-configured.
    """
    from crewai import Crew
    
    crew = Crew(agents=agents, tasks=tasks, **crew_kwargs)
    monitor = setup_attest_for_crew(crew, crew_name=crew_kwargs.get('crew_name', 'monitored_crew'))
    
    return crew


def validate_task_output(output: str, rules: List[str]) -> Dict[str, Any]:
    """
    Validate task output against defined rules.
    """
    results = {
        "passed": [],
        "failed": [],
        "warnings": []
    }
    
    for rule in rules:
        if rule == "contains_data":
            if len(output.strip()) > 0 {
                results["passed"].append(rule)
            } else {
                results["failed"].append(rule)
        }
        
        elif rule == "no_pii":
            import re
            email_pattern = r'\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b'
            if re.search(email_pattern, output) {
                results["failed"].append(rule)
            } else {
                results["passed"].append(rule)
            }
        
        elif rule == "min_length":
            if len(output) >= 100 {
                results["passed"].append(rule)
            } else {
                results["warnings"].append(f"{rule}: output is {len(output)} chars")
            }
    
    return results
`
}

func SetupCrewAI(framework string) error {
	err := os.MkdirAll("attest", 0755)
	if err != nil {
		return err
	}

	outputPath := filepath.Join("attest", "attest_crew_setup.py")
	content := GetCrewAITemplate()
	return os.WriteFile(outputPath, []byte(content), 0644)
}

func GenerateCrewAIWorkflow() string {
	return `name: Attest Validation

on:
  push:
    branches: [ main, develop ]
  pull_request:
    branches: [ main ]

jobs:
  validate:
    runs-on: ubuntu-latest
    
    steps:
      - uses: actions/checkout@v4
      
      - name: Set up Python
        uses: actions/setup-python@v4
        with:
          python-version: '3.11'
      
      - name: Install dependencies
        run: |
          python -m pip install --upgrade pip
          pip install -r requirements.txt || true
          pip install crewai
          pip install attest
      
      - name: Run Attest validation
        run: attest validate --ci
        env:
          ATTEST_API_KEY: ${{ secrets.ATTEST_API_KEY }}
      
      - name: Upload results
        uses: actions/upload-artifact@v3
        if: always()
        with:
          name: attest-results
          path: attest-results/
`
}
