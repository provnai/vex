package instrument

import (
	"sync"
	"time"

	"github.com/google/uuid"
)

type Tracer struct {
	mu         sync.Mutex
	isRunning  bool
	sessionID  string
	actions    []Action
	captureLLM bool
	captureFS  bool
	captureNet bool
}

type Action struct {
	ID        string                 `json:"id"`
	Type      string                 `json:"type"`
	Timestamp time.Time              `json:"timestamp"`
	Details   map[string]interface{} `json:"details"`
}

func NewTracer() *Tracer {
	return &Tracer{
		actions: make([]Action, 0),
	}
}

func (t *Tracer) Start(sessionID string, captureLLM, captureFS, captureNet bool) {
	t.mu.Lock()
	defer t.mu.Unlock()

	t.sessionID = sessionID
	t.isRunning = true
	t.captureLLM = captureLLM
	t.captureFS = captureFS
	t.captureNet = captureNet
}

func (t *Tracer) Stop() {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.isRunning = false
}

func (t *Tracer) IsRunning() bool {
	t.mu.Lock()
	defer t.mu.Unlock()
	return t.isRunning
}

func (t *Tracer) AddAction(action Action) {
	t.mu.Lock()
	defer t.mu.Unlock()

	action.ID = uuid.New().String()
	action.Timestamp = time.Now()
	t.actions = append(t.actions, action)
}

func (t *Tracer) GetActions() []Action {
	t.mu.Lock()
	defer t.mu.Unlock()

	result := make([]Action, len(t.actions))
	copy(result, t.actions)
	return result
}

func (t *Tracer) ClearActions() {
	t.mu.Lock()
	defer t.mu.Unlock()
	t.actions = make([]Action, 0)
}

func (t *Tracer) TraceLLMCall(provider, model, prompt string, response string, cost float64) {
	if !t.IsRunning() || !t.captureLLM {
		return
	}

	t.AddAction(Action{
		Type: "llm",
		Details: map[string]interface{}{
			"provider": provider,
			"model":    model,
			"prompt":   prompt,
			"response": response,
			"cost":     cost,
		},
	})
}

func (t *Tracer) TraceFileOperation(op, path string, content string) {
	if !t.IsRunning() || !t.captureFS {
		return
	}

	t.AddAction(Action{
		Type: "file",
		Details: map[string]interface{}{
			"operation": op,
			"path":      path,
			"content":   content,
		},
	})
}

func (t *Tracer) TraceNetworkRequest(method, url, body, response string) {
	if !t.IsRunning() || !t.captureNet {
		return
	}

	t.AddAction(Action{
		Type: "network",
		Details: map[string]interface{}{
			"method":   method,
			"url":      url,
			"request":  body,
			"response": response,
		},
	})
}

func (t *Tracer) TraceExec(command string, args []string, output string, errMsg string) {
	if !t.IsRunning() {
		return
	}

	t.AddAction(Action{
		Type: "exec",
		Details: map[string]interface{}{
			"command": command,
			"args":    args,
			"output":  output,
			"error":   errMsg,
		},
	})
}

func (t *Tracer) GetSessionSummary() map[string]interface{} {
	actions := t.GetActions()

	counts := map[string]int{}
	for _, a := range actions {
		counts[a.Type]++
	}

	return map[string]interface{}{
		"session_id": t.sessionID,
		"total":      len(actions),
		"by_type":    counts,
	}
}

func (t *Tracer) GetSessionID() string {
	return t.sessionID
}
