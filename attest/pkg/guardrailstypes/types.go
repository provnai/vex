package guardrailstypes

import (
	"context"
	"time"
)

type PolicyResult struct {
	PolicyID   string
	PolicyName string
	Passed     bool
	Severity   Severity
	Message    string
	Action     Action
	RiskLevel  RiskLevel
	Metadata   map[string]interface{}
}

type Severity string

const (
	SeverityInfo     Severity = "info"
	SeverityWarning  Severity = "warning"
	SeverityCritical Severity = "critical"
	SeverityBlocked  Severity = "blocked"
)

type Action string

const (
	ActionAllow   Action = "allow"
	ActionWarn    Action = "warn"
	ActionConfirm Action = "confirm"
	ActionBlock   Action = "block"
)

type RiskLevel string

const (
	RiskLevelLow      RiskLevel = "low"
	RiskLevelMedium   RiskLevel = "medium"
	RiskLevelHigh     RiskLevel = "high"
	RiskLevelCritical RiskLevel = "critical"
)

type Operation struct {
	ID            string
	Type          string
	Command       string
	Args          []string
	Env           map[string]string
	WorkingDir    string
	RiskLevel     RiskLevel
	EstimatedCost float64
	Metadata      map[string]interface{}
}

type Policy interface {
	ID() string
	Name() string
	Description() string
	Evaluate(ctx context.Context, op *Operation) (*PolicyResult, error)
	IsEnabled() bool
	SetEnabled(enabled bool)
}

type Checkpoint struct {
	ID          string
	OperationID string
	CreatedAt   time.Time
	Type        string
	Data        map[string]interface{}
	FileStates  []FileState
	DBStates    []DBState
	Size        int64
}

type FileState struct {
	Path        string
	Hash        string
	Content     []byte
	Exists      bool
	Permissions uint32
	ModTime     time.Time
}

type DBState struct {
	TableName string
	Records   map[string]interface{}
	Query     string
}

type RollbackResult struct {
	CheckpointID  string
	Success       bool
	RestoredFiles int
	RestoredDB    int
	Errors        []error
	Duration      time.Duration
}

type GuardrailViolationError struct {
	PolicyID   string
	PolicyName string
	Operation  string
	Message    string
	Severity   Severity
}

func (e *GuardrailViolationError) Error() string {
	return e.Message
}
