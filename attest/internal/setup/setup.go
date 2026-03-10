package setup

import (
	"fmt"
	"os"
	"path/filepath"
)

// Template files are embedded directly as functions below

func CreateConfig(framework string) (string, error) {
	configContent := generateConfig(framework)
	configPath := "attest.yaml"

	err := os.WriteFile(configPath, []byte(configContent), 0644)
	if err != nil {
		return "", err
	}

	return configPath, nil
}

func generateConfig(framework string) string {
	return fmt.Sprintf(`# Attest Configuration
version: "1.0"

# Framework settings
framework: %s

# Validation rules
validation:
  # Check for prompt injection vulnerabilities
  prompt_injection: true
  
  # Validate agent outputs
  output_validation: true
  
  # Monitor tool usage
  tool_monitoring: true
  
  # Track token usage and costs
  cost_tracking: true

# Test settings
testing:
  # Run tests on every commit
  run_on_commit: true
  
  # Required test coverage
  min_coverage: 80
  
  # Test scenarios
  scenarios:
    - name: basic_functionality
      description: "Basic agent functionality test"
    - name: edge_cases
      description: "Edge case handling"
    - name: safety_checks
      description: "Safety and security validation"

# Monitoring
monitoring:
  # Log all agent interactions
  logging: true
  
  # Export metrics
  metrics: true
  
  # Alert on anomalies
  alerting: true

# Integration settings
integration:
  # Git hooks
  git_hooks: true
  
  # CI/CD integration
  ci_integration: true
  
  # IDE plugins
  ide_support: true
`, framework)
}

func InstallFrameworkHooks(framework string) (string, error) {
	var templateContent string
	var outputFile string

	switch framework {
	case "langchain":
		outputFile = "attest_callback.py"
		templateContent = getLangChainTemplate()
	case "autogen":
		outputFile = "attest_autogen_setup.py"
		templateContent = getAutoGenTemplate()
	case "crewai":
		outputFile = "attest_crew_setup.py"
		templateContent = getCrewAITemplate()
	case "llamaindex":
		outputFile = "attest_llamaindex.py"
		templateContent = getLlamaIndexTemplate()
	default:
		return "", fmt.Errorf("unsupported framework: %s", framework)
	}

	// Create .attest directory if it doesn't exist
	err := os.MkdirAll(".attest", 0755)
	if err != nil {
		return "", err
	}

	outputPath := filepath.Join(".attest", outputFile)
	err = os.WriteFile(outputPath, []byte(templateContent), 0644)
	if err != nil {
		return "", err
	}

	return outputPath, nil
}

func SetupGitHooks() error {
	// Check if we're in a git repository
	if _, err := os.Stat(".git"); os.IsNotExist(err) {
		return fmt.Errorf("not a git repository")
	}

	hooksDir := filepath.Join(".git", "hooks")

	// Create pre-commit hook
	preCommitHook := `#!/bin/bash
# Attest Pre-Commit Hook
echo "Running Attest validation..."

# Check if attest is installed
if ! command -v attest &> /dev/null; then
    echo "Attest not found. Install with: pip install attest"
    exit 0
fi

# Run attest validation
attest validate

# Exit with attest's exit code
exit $?
`

	preCommitPath := filepath.Join(hooksDir, "pre-commit")
	err := os.WriteFile(preCommitPath, []byte(preCommitHook), 0755)
	if err != nil {
		return err
	}

	// Create post-commit hook for monitoring
	postCommitHook := `#!/bin/bash
# Attest Post-Commit Hook
echo "Sending metrics to Attest dashboard..."

# Optional: Send commit data for analysis
if command -v attest &> /dev/null; then
    attest log-commit --silent &
fi
`

	postCommitPath := filepath.Join(hooksDir, "post-commit")
	return os.WriteFile(postCommitPath, []byte(postCommitHook), 0755)
}

func SetupCITemplates(framework string) (string, error) {
	// Create .github/workflows directory
	workflowDir := filepath.Join(".github", "workflows")
	err := os.MkdirAll(workflowDir, 0755)
	if err != nil {
		return "", err
	}

	// Generate GitHub Actions workflow
	workflowContent := generateGitHubActionsWorkflow(framework)
	workflowPath := filepath.Join(workflowDir, "attest.yml")

	err = os.WriteFile(workflowPath, []byte(workflowContent), 0644)
	if err != nil {
		return "", err
	}

	return workflowPath, nil
}

func generateGitHubActionsWorkflow(framework string) string {
	frameworkSetup := ""
	switch framework {
	case "langchain":
		frameworkSetup = `
      - name: Install LangChain dependencies
        run: pip install langchain langchain-openai`
	case "autogen":
		frameworkSetup = `
      - name: Install AutoGen dependencies
        run: pip install pyautogen`
	case "crewai":
		frameworkSetup = `
      - name: Install CrewAI dependencies
        run: pip install crewai`
	case "llamaindex":
		frameworkSetup = `
      - name: Install LlamaIndex dependencies
        run: pip install llama-index`
	}

	return fmt.Sprintf(`name: Attest Validation

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
          pip install -r requirements.txt || true%s
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
`, frameworkSetup)
}

func getLangChainTemplate() string {
	return `"""
Attest Callback Handler for LangChain
Import this in your LangChain application to enable monitoring and validation.

Usage:
    from attest.attest_callback import AttestCallback
    from langchain.callbacks.base import CallbackManager
    
    callback = AttestCallback()
    llm = ChatOpenAI(callbacks=[callback])
"""

from typing import Any, Dict, List, Optional
from langchain.callbacks.base import BaseCallbackHandler
from langchain.schema import LLMResult, AgentAction, AgentFinish
import json
import time
import requests


class AttestCallback(BaseCallbackHandler):
    """Callback handler for Attest integration with LangChain."""
    
    def __init__(self, api_key: Optional[str] = None, endpoint: Optional[str] = None):
        super().__init__()
        self.api_key = api_key or self._get_api_key()
        self.endpoint = endpoint or "https://api.attest.dev/v1"
        self.session_id = self._generate_session_id()
        self.start_time = None
        
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
            # Fail silently to not interrupt the application
            pass
    
    def on_llm_start(
        self, 
        serialized: Dict[str, Any], 
        prompts: List[str], 
        **kwargs: Any
    ) -> None:
        """Run when LLM starts."""
        self.start_time = time.time()
        self._send_event("llm_start", {
            "prompts": prompts,
            "model": serialized.get("name", "unknown")
        })
    
    def on_llm_end(self, response: LLMResult, **kwargs: Any) -> None:
        """Run when LLM ends."""
        duration = time.time() - self.start_time if self.start_time else 0
        
        # Extract token usage
        token_usage = response.llm_output.get("token_usage", {}) if response.llm_output else {}
        
        self._send_event("llm_end", {
            "duration": duration,
            "token_usage": token_usage,
            "generations": len(response.generations)
        })
    
    def on_llm_error(self, error: Exception, **kwargs: Any) -> None:
        """Run when LLM errors."""
        self._send_event("llm_error", {
            "error": str(error),
            "error_type": type(error).__name__
        })
    
    def on_tool_start(
        self, 
        serialized: Dict[str, Any], 
        input_str: str, 
        **kwargs: Any
    ) -> None:
        """Run when tool starts."""
        self._send_event("tool_start", {
            "tool": serialized.get("name", "unknown"),
            "input": input_str
        })
    
    def on_tool_end(self, output: str, **kwargs: Any) -> None:
        """Run when tool ends."""
        self._send_event("tool_end", {
            "output": output[:1000]  # Truncate long outputs
        })
    
    def on_tool_error(self, error: Exception, **kwargs: Any) -> None:
        """Run when tool errors."""
        self._send_event("tool_error", {
            "error": str(error)
        })
    
    def on_agent_action(self, action: AgentAction, **kwargs: Any) -> None:
        """Run on agent action."""
        self._send_event("agent_action", {
            "tool": action.tool,
            "tool_input": str(action.tool_input),
            "log": action.log
        })
    
    def on_agent_finish(self, finish: AgentFinish, **kwargs: Any) -> None:
        """Run on agent finish."""
        self._send_event("agent_finish", {
            "output": finish.return_values,
            "log": finish.log
        })
    
    def on_chain_start(
        self, 
        serialized: Dict[str, Any], 
        inputs: Dict[str, Any], 
        **kwargs: Any
    ) -> None:
        """Run when chain starts."""
        self._send_event("chain_start", {
            "chain_type": serialized.get("name", "unknown"),
            "inputs": {k: str(v)[:500] for k, v in inputs.items()}
        })
    
    def on_chain_end(self, outputs: Dict[str, Any], **kwargs: Any) -> None:
        """Run when chain ends."""
        self._send_event("chain_end", {
            "outputs": {k: str(v)[:500] for k, v in outputs.items()}
        })


# Convenience function for quick setup
def enable_attest(**kwargs):
    """Quick setup function to enable Attest monitoring."""
    return AttestCallback(**kwargs)
`
}

func getAutoGenTemplate() string {
	return `"""
Attest Integration for AutoGen
Import this module to enable Attest monitoring for AutoGen teams.

Usage:
    from attest.attest_autogen_setup import setup_attest_monitoring
    
    # Setup monitoring for your AutoGen team
    setup_attest_monitoring(team)
"""

from typing import Any, Dict, List, Optional, Callable
import json
import time
import requests
from functools import wraps


class AttestAutoGenMonitor:
    """Monitor for AutoGen conversations and agents."""
    
    def __init__(self, api_key: Optional[str] = None, endpoint: Optional[str] = None):
        self.api_key = api_key or self._get_api_key()
        self.endpoint = endpoint or "https://api.attest.dev/v1"
        self.session_id = self._generate_session_id()
        self.conversation_data = []
        
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
    
    def monitor_conversation(self, conversation_history: List[Dict[str, Any]]):
        """Monitor conversation history."""
        self._send_event("conversation", {
            "messages": len(conversation_history),
            "conversation": conversation_history[-10:]  # Last 10 messages
        })
    
    def monitor_agent_message(
        self, 
        agent_name: str, 
        message: str, 
        role: str = "assistant",
        metadata: Optional[Dict[str, Any]] = None
    ):
        """Monitor individual agent message."""
        self._send_event("agent_message", {
            "agent": agent_name,
            "role": role,
            "message_length": len(message),
            "message_preview": message[:500],
            "metadata": metadata or {}
        })
    
    def monitor_function_call(
        self, 
        function_name: str, 
        arguments: Dict[str, Any],
        result: Any,
        duration: Optional[float] = None
    ):
        """Monitor function/tool calls."""
        self._send_event("function_call", {
            "function": function_name,
            "arguments": arguments,
            "result_preview": str(result)[:500],
            "duration": duration
        })
    
    def monitor_termination(self, reason: str, summary: Optional[str] = None):
        """Monitor conversation termination."""
        self._send_event("termination", {
            "reason": reason,
            "summary": summary
        })
    
    def validate_output(self, output: str, validation_rules: Optional[List[str]] = None):
        """Validate agent output against rules."""
        self._send_event("validation_request", {
            "output_preview": output[:1000],
            "rules": validation_rules or []
        })


def setup_attest_monitoring(team_or_group: Any, **kwargs) -> AttestAutoGenMonitor:
    """
    Setup Attest monitoring for an AutoGen team or group chat.
    
    Args:
        team_or_group: AutoGen GroupChat or GroupChatManager instance
        **kwargs: Additional configuration options
    
    Returns:
        AttestAutoGenMonitor instance
    """
    monitor = AttestAutoGenMonitor(**kwargs)
    
    # Try to wrap the team/chat methods for monitoring
    try:
        _wrap_autogen_chat(team_or_group, monitor)
    except Exception as e:
        print(f"Warning: Could not fully wrap AutoGen team: {e}")
    
    return monitor


def _wrap_autogen_chat(team: Any, monitor: AttestAutoGenMonitor):
    """Wrap AutoGen chat methods with monitoring."""
    # Wrap run_chat if it exists
    if hasattr(team, 'run_chat'):
        original_run_chat = team.run_chat
        
        @wraps(original_run_chat)
        def monitored_run_chat(*args, **kwargs):
            start_time = time.time()
            
            # Capture the result
            result = original_run_chat(*args, **kwargs)
            
            # Monitor the conversation
            if hasattr(team, 'messages'):
                monitor.monitor_conversation(team.messages)
            
            duration = time.time() - start_time
            monitor._send_event("chat_complete", {
                "duration": duration,
                "message_count": len(team.messages) if hasattr(team, 'messages') else 0
            })
            
            return result
        
        team.run_chat = monitored_run_chat
    
    # Wrap GroupChatManager's run_chat if available
    if hasattr(team, 'groupchat') and team.groupchat:
        _wrap_autogen_chat(team.groupchat, monitor)


def create_monitored_agent(
    original_create_agent_func: Callable,
    monitor: AttestAutoGenMonitor,
    *args,
    **kwargs
):
    """
    Create an AutoGen agent with Attest monitoring built-in.
    
    Usage:
        from autogen import AssistantAgent
        from attest.attest_autogen_setup import create_monitored_agent, AttestAutoGenMonitor
        
        monitor = AttestAutoGenMonitor()
        agent = create_monitored_agent(AssistantAgent, monitor, name="assistant", ...)
    """
    agent = original_create_agent_func(*args, **kwargs)
    
    # Wrap the agent's message sending
    if hasattr(agent, 'send'):
        original_send = agent.send
        
        @wraps(original_send)
        def monitored_send(message: str, *send_args, **send_kwargs):
            # Monitor the outgoing message
            monitor.monitor_agent_message(
                agent_name=getattr(agent, 'name', 'unknown'),
                message=message,
                role=getattr(agent, 'system_message', '')[:100]
            )
            
            return original_send(message, *send_args, **send_kwargs)
        
        agent.send = monitored_send
    
    return agent


# Decorator for custom agent functions
def attest_monitored(monitor: Optional[AttestAutoGenMonitor] = None):
    """Decorator to monitor custom agent functions."""
    def decorator(func: Callable):
        nonlocal monitor
        if monitor is None:
            monitor = AttestAutoGenMonitor()
        
        @wraps(func)
        def wrapper(*args, **kwargs):
            start_time = time.time()
            
            try:
                result = func(*args, **kwargs)
                duration = time.time() - start_time
                
                monitor.monitor_function_call(
                    function_name=func.__name__,
                    arguments={"args": str(args), "kwargs": str(kwargs)},
                    result=result,
                    duration=duration
                )
                
                return result
            except Exception as e:
                monitor._send_event("function_error", {
                    "function": func.__name__,
                    "error": str(e)
                })
                raise
        
        return wrapper
    return decorator
`
}

func getCrewAITemplate() string {
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
    
    def validate_crew_output(self, output: str, criteria: Optional[List[str]] = None):
        """Request validation of crew output."""
        self._send_event("validation_request", {
            "output_preview": output[:2000],
            "validation_criteria": criteria or []
        })
    
    def log_agent_thought(self, agent_name: str, thought: str):
        """Log agent's thought process."""
        self._send_event("agent_thought", {
            "agent": agent_name,
            "thought_preview": thought[:1000]
        })


def setup_attest_for_crew(crew: Any, **kwargs) -> AttestCrewMonitor:
    """
    Setup Attest monitoring for a CrewAI crew.
    
    Args:
        crew: CrewAI Crew instance
        **kwargs: Configuration options (api_key, endpoint, crew_name)
    
    Returns:
        AttestCrewMonitor instance
    """
    crew_name = kwargs.get('crew_name', getattr(crew, 'name', 'unnamed_crew'))
    monitor = AttestCrewMonitor(crew_name=crew_name, **kwargs)
    
    # Store original kickoff method
    if hasattr(crew, 'kickoff'):
        original_kickoff = crew.kickoff
        
        @wraps(original_kickoff)
        def monitored_kickoff(*args, **kickoff_kwargs):
            start_time = time.time()
            
            # Get agents and tasks
            agents = getattr(crew, 'agents', [])
            tasks = getattr(crew, 'tasks', [])
            
            # Monitor crew start
            monitor.monitor_crew_start(agents, tasks)
            
            try:
                # Execute original kickoff
                result = original_kickoff(*args, **kickoff_kwargs)
                
                # Monitor completion
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
    
    # Wrap individual agents if possible
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
            
            # Monitor task start
            monitor.monitor_task_start(task, agent)
            
            # Execute task
            result = original_execute(task, *args, **kwargs)
            
            # Monitor task completion
            duration = time.time() - start_time
            monitor.monitor_task_complete(task, agent, result, duration)
            
            return result
        
        agent.execute_task = monitored_execute
    
    # Wrap tool usage
    if hasattr(agent, 'tools'):
        original_tools = agent.tools
        monitored_tools = []
        
        for tool in original_tools:
            if hasattr(tool, '_run'):
                original_run = tool._run
                
                @wraps(original_run)
                def monitored_tool_run(*args, **kwargs):
                    start_time = time.time()
                    result = original_run(*args, **kwargs)
                    duration = time.time() - start_time
                    
                    monitor.monitor_tool_usage(
                        tool_name=getattr(tool, 'name', 'unknown'),
                        inputs=dict(zip(getattr(tool, 'args', {}).keys(), args)),
                        output=result,
                        duration=duration
                    )
                    
                    return result
                
                tool._run = monitored_tool_run
            monitored_tools.append(tool)
        
        agent.tools = monitored_tools


def create_monitored_crew(
    agents: List[Any],
    tasks: List[Any],
    monitor: Optional[AttestCrewMonitor] = None,
    **crew_kwargs
):
    """
    Create a CrewAI crew with Attest monitoring pre-configured.
    
    Usage:
        from crewai import Crew, Agent, Task
        from attest.attest_crew_setup import create_monitored_crew
        
        crew = create_monitored_crew(
            agents=[agent1, agent2],
            tasks=[task1, task2],
            crew_name="my_crew"
        )
        
        result = crew.kickoff()
    """
    from crewai import Crew
    
    crew = Crew(agents=agents, tasks=tasks, **crew_kwargs)
    monitor = setup_attest_for_crew(crew, crew_name=crew_kwargs.get('crew_name', 'monitored_crew'))
    
    return crew


# Validation helpers
def validate_task_output(output: str, rules: List[str]) -> Dict[str, Any]:
    """
    Validate task output against defined rules.
    
    Args:
        output: The output to validate
        rules: List of validation rules (e.g., ["contains_data", "no_pii"])
    
    Returns:
        Dictionary with validation results
    """
    results = {
        "passed": [],
        "failed": [],
        "warnings": []
    }
    
    for rule in rules:
        if rule == "contains_data":
            if len(output.strip()) > 0:
                results["passed"].append(rule)
            else:
                results["failed"].append(rule)
        
        elif rule == "no_pii":
            # Basic PII detection (email, phone)
            import re
            email_pattern = r'\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b'
            if re.search(email_pattern, output):
                results["failed"].append(rule)
            else:
                results["passed"].append(rule)
        
        elif rule == "min_length":
            if len(output) >= 100:
                results["passed"].append(rule)
            else:
                results["warnings"].append(f"{rule}: output is {len(output)} chars")
    
    return results
`
}

func getLlamaIndexTemplate() string {
	return `"""
Attest Callback Handler for LlamaIndex
Import this in your LlamaIndex application to enable monitoring and validation.

Usage:
    from .attest_llamaindex import AttestCallbackHandler
    from llama_index.core import Settings

    handler = AttestCallbackHandler()
    Settings.callback_manager = CallbackManager([handler])
"""

from typing import Any, Dict, List, Optional
from llama_index.core.callbacks.base import BaseCallbackHandler
from llama_index.core.callbacks.schema import CBEventType, EventPayload
import json
import time
import requests


class AttestCallbackHandler(BaseCallbackHandler):
    """Callback handler for Attest integration with LlamaIndex."""
    
    def __init__(self, api_key: Optional[str] = None, endpoint: Optional[str] = None):
        super().__init__([], [])
        self.api_key = api_key or self._get_api_key()
        self.endpoint = endpoint or "https://api.attest.dev/v1"
        self.session_id = self._generate_session_id()
        self.start_time = None
        
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
    
    def on_event_start(
        self,
        event_type: CBEventType,
        payload: Optional[Dict[str, Any]] = None,
        event_id: str = "",
        **kwargs: Any
    ) -> str:
        """Run when an event starts."""
        self.start_time = time.time()
        
        if event_type == CBEventType.LLM:
            prompt = payload.get(EventPayload.PROMPT, "") if payload else ""
            self._send_event("llm_start", {
                "prompt": prompt[:1000],
                "event_id": event_id
            })
        
        return event_id
    
    def on_event_end(
        self,
        event_type: CBEventType,
        payload: Optional[Dict[str, Any]] = None,
        event_id: str = "",
        **kwargs: Any
    ) -> None:
        """Run when an event ends."""
        duration = time.time() - self.start_time if self.start_time else 0
        
        if event_type == CBEventType.LLM:
            response = payload.get(EventPayload.RESPONSE, "") if payload else ""
            self._send_event("llm_end", {
                "duration": duration,
                "response_preview": response[:500],
                "event_id": event_id
            })


def setup_attest_callback(api_key: Optional[str] = None) -> AttestCallbackHandler:
    """Setup Attest callback handler for LlamaIndex."""
    handler = AttestCallbackHandler(api_key=api_key)
    return handler
`
}
