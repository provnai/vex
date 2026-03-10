package guardrails

import (
	"github.com/provnai/attest/pkg/guardrailstypes"
)

type PolicyResult = guardrailstypes.PolicyResult
type Severity = guardrailstypes.Severity
type Action = guardrailstypes.Action
type RiskLevel = guardrailstypes.RiskLevel
type Operation = guardrailstypes.Operation
type Policy = guardrailstypes.Policy
type Checkpoint = guardrailstypes.Checkpoint
type FileState = guardrailstypes.FileState
type DBState = guardrailstypes.DBState
type RollbackResult = guardrailstypes.RollbackResult
type GuardrailViolationError = guardrailstypes.GuardrailViolationError

const (
	SeverityInfo      = guardrailstypes.SeverityInfo
	SeverityWarning   = guardrailstypes.SeverityWarning
	SeverityCritical  = guardrailstypes.SeverityCritical
	SeverityBlocked   = guardrailstypes.SeverityBlocked
	ActionAllow       = guardrailstypes.ActionAllow
	ActionWarn        = guardrailstypes.ActionWarn
	ActionConfirm     = guardrailstypes.ActionConfirm
	ActionBlock       = guardrailstypes.ActionBlock
	RiskLevelLow      = guardrailstypes.RiskLevelLow
	RiskLevelMedium   = guardrailstypes.RiskLevelMedium
	RiskLevelHigh     = guardrailstypes.RiskLevelHigh
	RiskLevelCritical = guardrailstypes.RiskLevelCritical
)
