// pkg/cost/middleware.go - intercepts API calls for cost tracking

package cost

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"strings"
	"time"
)

// Middleware handles HTTP interception for cost tracking
type Middleware struct {
	tracker *Tracker
	limiter *Limiter
	db      *sql.DB
}

// NewMiddleware creates cost tracking middleware
func NewMiddleware(db *sql.DB) *Middleware {
	return &Middleware{
		tracker: NewTracker(db),
		limiter: NewLimiter(db),
		db:      db,
	}
}

// APICallInfo contains information about an intercepted API call
type APICallInfo struct {
	Provider     string
	Model        string
	InputTokens  int64
	OutputTokens int64
	Latency      time.Duration
	Success      bool
	Error        string
}

// InterceptedTransport wraps http.RoundTripper with cost tracking
type InterceptedTransport struct {
	Base    http.RoundTripper
	Tracker *Tracker
	Limiter *Limiter
	RunID   string
}

// RoundTrip implements http.RoundTripper with cost tracking
func (t *InterceptedTransport) RoundTrip(req *http.Request) (*http.Response, error) {
	// Check budget before making request
	if err := t.Limiter.PreFlightCheck(req.Context()); err != nil {
		return nil, err
	}

	start := time.Now()

	// Read request body for token counting
	var reqBody []byte
	if req.Body != nil {
		reqBody, _ = io.ReadAll(req.Body)
		req.Body = io.NopCloser(strings.NewReader(string(reqBody)))
	}

	// Make the actual request
	resp, err := t.Base.RoundTrip(req)
	latency := time.Since(start)

	// Extract model and tokens from request/response
	info := t.extractCallInfo(req, resp, reqBody, err, latency)

	// Record the cost if we have valid data
	if info.Model != "" && (info.InputTokens > 0 || info.OutputTokens > 0) {
		_, recordErr := t.Tracker.RecordAPICall(
			req.Context(),
			info.Model,
			info.Provider,
			info.InputTokens,
			info.OutputTokens,
			t.RunID,
		)
		if recordErr != nil {
			// Log but don't fail the request
			fmt.Printf("Failed to record cost: %v\n", recordErr)
		}
	}

	return resp, err
}

// extractCallInfo extracts model and token information from API calls
func (t *InterceptedTransport) extractCallInfo(req *http.Request, resp *http.Response, reqBody []byte, err error, latency time.Duration) *APICallInfo {
	info := &APICallInfo{
		Latency: latency,
		Success: err == nil && resp != nil && resp.StatusCode < 400,
	}

	if err != nil {
		info.Error = err.Error()
	}

	// Detect provider from URL
	url := req.URL.String()
	switch {
	case strings.Contains(url, "api.openai.com"):
		info.Provider = "openai"
		t.extractOpenAIInfo(req, resp, reqBody, info)
	case strings.Contains(url, "api.anthropic.com"):
		info.Provider = "anthropic"
		t.extractAnthropicInfo(req, resp, reqBody, info)
	case strings.Contains(url, "generativelanguage.googleapis.com"):
		info.Provider = "google"
		t.extractGoogleInfo(req, resp, reqBody, info)
	}

	return info
}

// extractOpenAIInfo extracts info from OpenAI API calls
func (t *InterceptedTransport) extractOpenAIInfo(req *http.Request, resp *http.Response, reqBody []byte, info *APICallInfo) {
	// Parse request body for model and input
	if len(reqBody) > 0 {
		var reqData map[string]interface{}
		if err := json.Unmarshal(reqBody, &reqData); err == nil {
			if model, ok := reqData["model"].(string); ok {
				info.Model = model
			}

			// Estimate input tokens from messages
			if messages, ok := reqData["messages"].([]interface{}); ok {
				info.InputTokens = int64(t.estimateTokensFromMessages(messages))
			}
		}
	}

	// Parse response for actual token usage
	if resp != nil && resp.Body != nil {
		respBody, _ := io.ReadAll(resp.Body)
		resp.Body = io.NopCloser(strings.NewReader(string(respBody)))

		var respData map[string]interface{}
		if err := json.Unmarshal(respBody, &respData); err == nil {
			if usage, ok := respData["usage"].(map[string]interface{}); ok {
				if prompt, ok := usage["prompt_tokens"].(float64); ok {
					info.InputTokens = int64(prompt)
				}
				if completion, ok := usage["completion_tokens"].(float64); ok {
					info.OutputTokens = int64(completion)
				}
			}
		}
	}
}

// extractAnthropicInfo extracts info from Anthropic API calls
func (t *InterceptedTransport) extractAnthropicInfo(req *http.Request, resp *http.Response, reqBody []byte, info *APICallInfo) {
	if len(reqBody) > 0 {
		var reqData map[string]interface{}
		if err := json.Unmarshal(reqBody, &reqData); err == nil {
			if model, ok := reqData["model"].(string); ok {
				info.Model = model
			}

			// Estimate from messages
			if messages, ok := reqData["messages"].([]interface{}); ok {
				info.InputTokens = int64(t.estimateTokensFromMessages(messages))
			}
			// Also check max_tokens for output estimate
			if maxTokens, ok := reqData["max_tokens"].(float64); ok {
				// Rough estimate: assume 80% of max tokens used on average
				info.OutputTokens = int64(maxTokens * 0.8)
			}
		}
	}

	// Parse response for actual usage
	if resp != nil && resp.Body != nil {
		respBody, _ := io.ReadAll(resp.Body)
		resp.Body = io.NopCloser(strings.NewReader(string(respBody)))

		var respData map[string]interface{}
		if err := json.Unmarshal(respBody, &respData); err == nil {
			if usage, ok := respData["usage"].(map[string]interface{}); ok {
				if input, ok := usage["input_tokens"].(float64); ok {
					info.InputTokens = int64(input)
				}
				if output, ok := usage["output_tokens"].(float64); ok {
					info.OutputTokens = int64(output)
				}
			}
		}
	}
}

// extractGoogleInfo extracts info from Google AI API calls
func (t *InterceptedTransport) extractGoogleInfo(req *http.Request, resp *http.Response, reqBody []byte, info *APICallInfo) {
	if len(reqBody) > 0 {
		var reqData map[string]interface{}
		if err := json.Unmarshal(reqBody, &reqData); err == nil {
			// Extract model from endpoint
			if contents, ok := reqData["contents"].([]interface{}); ok {
				info.InputTokens = int64(t.estimateTokensFromContents(contents))
			}
		}
	}

	// Parse response
	if resp != nil && resp.Body != nil {
		respBody, _ := io.ReadAll(resp.Body)
		resp.Body = io.NopCloser(strings.NewReader(string(respBody)))

		var respData map[string]interface{}
		if err := json.Unmarshal(respBody, &respData); err == nil {
			if usage, ok := respData["usageMetadata"].(map[string]interface{}); ok {
				if prompt, ok := usage["promptTokenCount"].(float64); ok {
					info.InputTokens = int64(prompt)
				}
				if candidates, ok := usage["candidatesTokenCount"].(float64); ok {
					info.OutputTokens = int64(candidates)
				}
			}
		}
	}
}

// estimateTokensFromMessages rough token estimation for OpenAI/Anthropic format
func (t *InterceptedTransport) estimateTokensFromMessages(messages []interface{}) int {
	tokens := 0
	for _, msg := range messages {
		if msgMap, ok := msg.(map[string]interface{}); ok {
			if content, ok := msgMap["content"].(string); ok {
				// Rough estimate: ~4 chars per token
				tokens += len(content) / 4
			}
			// Add tokens for message structure
			tokens += 4
		}
	}
	return tokens
}

// estimateTokensFromContents estimates tokens for Google format
func (t *InterceptedTransport) estimateTokensFromContents(contents []interface{}) int {
	tokens := 0
	for _, content := range contents {
		if contentMap, ok := content.(map[string]interface{}); ok {
			if parts, ok := contentMap["parts"].([]interface{}); ok {
				for _, part := range parts {
					if partMap, ok := part.(map[string]interface{}); ok {
						if text, ok := partMap["text"].(string); ok {
							tokens += len(text) / 4
						}
					}
				}
			}
		}
	}
	return tokens
}

// WrapClient wraps an HTTP client with cost tracking
func (m *Middleware) WrapClient(client *http.Client, runID string) *http.Client {
	if client == nil {
		client = http.DefaultClient
	}

	// Generate runID if not provided
	if runID == "" {
		runID = generateRunID()
	}

	wrapped := &http.Client{
		Transport: &InterceptedTransport{
			Base:    client.Transport,
			Tracker: m.tracker,
			Limiter: m.limiter,
			RunID:   runID,
		},
		Timeout: client.Timeout,
	}

	// If no transport set, use default
	if wrapped.Transport.(*InterceptedTransport).Base == nil {
		wrapped.Transport.(*InterceptedTransport).Base = http.DefaultTransport
	}

	return wrapped
}

// CreateWrappedTransport creates a transport for use with any HTTP client
func (m *Middleware) CreateWrappedTransport(runID string) http.RoundTripper {
	if runID == "" {
		runID = generateRunID()
	}

	return &InterceptedTransport{
		Base:    http.DefaultTransport,
		Tracker: m.tracker,
		Limiter: m.limiter,
		RunID:   runID,
	}
}

// BudgetEnforcingRoundTripper returns a transport that enforces budgets
func (m *Middleware) BudgetEnforcingRoundTripper(runID string) http.RoundTripper {
	return m.CreateWrappedTransport(runID)
}

// generateRunID creates a unique run identifier
func generateRunID() string {
	b := make([]byte, 8)
	if _, err := rand.Read(b); err != nil {
		// Fallback to timestamp if entropy fails
		return fmt.Sprintf("%x", time.Now().UnixNano())
	}
	return hex.EncodeToString(b)
}

// TrackManualAPICall allows manual cost tracking for non-HTTP APIs
func (m *Middleware) TrackManualAPICall(ctx context.Context, provider, model string, inputTokens, outputTokens int64, runID string) error {
	// Check budget first
	if err := m.limiter.PreFlightCheck(ctx); err != nil {
		return err
	}

	// Record the cost
	_, err := m.tracker.RecordAPICall(ctx, model, provider, inputTokens, outputTokens, runID)
	return err
}
