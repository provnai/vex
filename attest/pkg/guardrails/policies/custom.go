package policies

import (
	"context"
	"fmt"
	"strings"

	"github.com/provnai/attest/pkg/guardrailstypes"
	"github.com/provnai/attest/pkg/policy"
)

// CustomPolicy wraps a generic policy.Policy to fit the guardrailstypes.Policy interface
type CustomPolicy struct {
	inner   *policy.Policy
	enabled bool
}

// NewCustomPolicy creates a new custom policy from a generic policy
func NewCustomPolicy(p *policy.Policy) *CustomPolicy {
	return &CustomPolicy{
		inner:   p,
		enabled: p.Enabled,
	}
}

func (p *CustomPolicy) ID() string {
	return p.inner.ID
}

func (p *CustomPolicy) Name() string {
	return p.inner.Name
}

func (p *CustomPolicy) Description() string {
	return p.inner.Description
}

func (p *CustomPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *CustomPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
	p.inner.Enabled = enabled
}

func (p *CustomPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	// Map guardrailstypes.Operation to policy.ActionContext
	actionCtx := policy.ActionContext{
		Type:           op.Type,
		Target:         op.Command + " " + strings.Join(op.Args, " "),
		Classification: fmt.Sprintf("%v", op.Metadata["classification"]),
		Environment:    fmt.Sprintf("%v", op.Env["ATTEST_ENV"]),
		RiskLevel:      string(op.RiskLevel),
	}

	// Evaluate the inner policy
	res := p.inner.Evaluate(actionCtx)

	// Map policy.PolicyResult back to guardrailstypes.PolicyResult
	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     !res.Matched,
		Severity:   guardrailstypes.Severity(res.Severity),
		Message:    res.Message,
		Action:     guardrailstypes.Action(res.Action),
		RiskLevel:  p.mapSeverityToRisk(res.Severity),
	}, nil
}

func (p *CustomPolicy) mapSeverityToRisk(s policy.PolicySeverity) guardrailstypes.RiskLevel {
	switch s {
	case policy.SeverityCritical:
		return guardrailstypes.RiskLevelCritical
	case policy.SeverityWarning:
		return guardrailstypes.RiskLevelHigh
	default:
		return guardrailstypes.RiskLevelMedium
	}
}

// LoadCustomPolicy loads a policy from a YAML file and wraps it
func LoadCustomPolicy(path string) (*CustomPolicy, error) {
	p, err := policy.LoadPolicyFromFile(path)
	if err != nil {
		return nil, err
	}
	return NewCustomPolicy(p), nil
}
