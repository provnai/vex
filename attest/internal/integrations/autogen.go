package integrations

import (
	"os"
	"path/filepath"
)

func GetAutoGenTemplate() string {
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
            "conversation": conversation_history[-10:]
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


def setup_attest_monitoring(team_or_group: Any, **kwargs) -> AttestAutoGenMonitor:
    """
    Setup Attest monitoring for an AutoGen team or group chat.
    """
    monitor = AttestAutoGenMonitor(**kwargs)
    
    try:
        _wrap_autogen_chat(team_or_group, monitor)
    except Exception as e:
        print(f"Warning: Could not fully wrap AutoGen team: {e}")
    
    return monitor


def _wrap_autogen_chat(team: Any, monitor: AttestAutoGenMonitor):
    """Wrap AutoGen chat methods with monitoring."""
    if hasattr(team, 'run_chat'):
        original_run_chat = team.run_chat
        
        @wraps(original_run_chat)
        def monitored_run_chat(*args, **kwargs):
            start_time = time.time()
            result = original_run_chat(*args, **kwargs)
            
            if hasattr(team, 'messages'):
                monitor.monitor_conversation(team.messages)
            
            duration = time.time() - start_time
            monitor._send_event("chat_complete", {
                "duration": duration,
                "message_count": len(team.messages) if hasattr(team, 'messages') else 0
            })
            
            return result
        
        team.run_chat = monitored_run_chat


def create_monitored_agent(
    original_create_agent_func: Callable,
    monitor: AttestAutoGenMonitor,
    *args,
    **kwargs
):
    """
    Create an AutoGen agent with Attest monitoring built-in.
    """
    agent = original_create_agent_func(*args, **kwargs)
    
    if hasattr(agent, 'send'):
        original_send = agent.send
        
        @wraps(original_send)
        def monitored_send(message: str, *send_args, **send_kwargs):
            monitor.monitor_agent_message(
                agent_name=getattr(agent, 'name', 'unknown'),
                message=message,
                role=getattr(agent, 'system_message', '')[:100]
            )
            return original_send(message, *send_args, **send_kwargs)
        
        agent.send = monitored_send
    
    return agent


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

func SetupAutoGen(framework string) error {
	err := os.MkdirAll("attest", 0755)
	if err != nil {
		return err
	}

	outputPath := filepath.Join("attest", "attest_autogen_setup.py")
	content := GetAutoGenTemplate()
	return os.WriteFile(outputPath, []byte(content), 0644)
}

func GenerateAutoGenWorkflow() string {
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
          pip install pyautogen
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
