package integrations

import (
	"os"
	"path/filepath"
)

func GetLangChainTemplate() string {
	return `"""
Attest Callback Handler for LangChain
Import this in your LangChain application to enable monitoring and validation.

WARNING: This integration is DEPRECATED and relies on a SaaS endpoint (api.attest.dev) which may not be available.
We recommend using the official Local Python SDK ("attest_client.py") which interfaces directly with the CLI.

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
            "output": output[:1000]
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


def enable_attest(**kwargs):
    """Quick setup function to enable Attest monitoring."""
    return AttestCallback(**kwargs)
`
}

func SetupLangChain(framework string) error {
	err := os.MkdirAll("attest", 0755)
	if err != nil {
		return err
	}

	outputPath := filepath.Join("attest", "attest_callback.py")
	content := GetLangChainTemplate()
	return os.WriteFile(outputPath, []byte(content), 0644)
}

func GenerateLangChainWorkflow() string {
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
          pip install langchain langchain-openai
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
