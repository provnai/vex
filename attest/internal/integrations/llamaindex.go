package integrations

import (
	"os"
	"path/filepath"
)

func GetLlamaIndexTemplate() string {
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
        self.events: List[Dict] = []
        
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
        
        elif event_type == CBEventType.RETRIEVE:
            query = payload.get(EventPayload.QUERY_STR, "") if payload else ""
            self._send_event("retrieve_start", {
                "query": query[:500],
                "event_id": event_id
            })
        
        elif event_type == CBEventType.NODE_PARSING:
            documents = payload.get(EventPayload.DOCUMENTS, []) if payload else []
            self._send_event("node_parsing", {
                "document_count": len(documents),
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
            token_usage = payload.get(EventPayload.TOKEN_USAGE, {}) if payload else {}
            
            self._send_event("llm_end", {
                "duration": duration,
                "response_preview": response[:500],
                "token_usage": token_usage,
                "event_id": event_id
            })
        
        elif event_type == CBEventType.RETRIEVE:
            nodes = payload.get(EventPayload.NODES, []) if payload else []
            self._send_event("retrieve_end", {
                "duration": duration,
                "nodes_retrieved": len(nodes),
                "event_id": event_id
            })
        
        elif event_type == CBEventType.QUERY:
            response = payload.get(EventPayload.RESPONSE, "") if payload else ""
            self._send_event("query_end", {
                "duration": duration,
                "response_length": len(response),
                "event_id": event_id
            })
    
    def on_error(self, error: Exception, **kwargs: Any) -> None:
        """Run when an error occurs."""
        self._send_event("error", {
            "error": str(error),
            "error_type": type(error).__name__
        })
    
    def start_trace(self, trace_id: Optional[str] = None) -> Optional[str]:
        """Start a trace."""
        trace_id = trace_id or self._generate_session_id()
        self._send_event("trace_start", {"trace_id": trace_id})
        return trace_id
    
    def end_trace(
        self,
        trace_id: Optional[str] = None,
        error: Optional[Exception] = None,
        **kwargs: Any
    ) -> None:
        """End a trace."""
        self._send_event("trace_end", {
            "trace_id": trace_id,
            "error": str(error) if error else None
        })


def setup_attest_callback(api_key: Optional[str] = None) -> AttestCallbackHandler:
    """
    Setup Attest callback handler for LlamaIndex.
    
    Usage:
        from llama_index.core import Settings
        from .attest_llamaindex import setup_attest_callback
        
        handler = setup_attest_callback()
        Settings.callback_manager = CallbackManager([handler])
    """
    handler = AttestCallbackHandler(api_key=api_key)
    return handler
`
}

func SetupLlamaIndex(framework string) error {
	err := os.MkdirAll(".attest", 0755)
	if err != nil {
		return err
	}

	outputPath := filepath.Join(".attest", "attest_llamaindex.py")
	content := GetLlamaIndexTemplate()
	return os.WriteFile(outputPath, []byte(content), 0644)
}

func GenerateLlamaIndexWorkflow() string {
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
          pip install llama-index
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
