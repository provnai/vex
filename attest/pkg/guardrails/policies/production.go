package policies

import (
	"context"
	"os"
	"strings"

	"github.com/provnai/attest/pkg/guardrailstypes"
)

type ProductionSafetyPolicy struct {
	enabled        bool
	isProduction   bool
	requireConfirm bool
	blockedInProd  []string
}

func NewProductionSafetyPolicy() *ProductionSafetyPolicy {
	return &ProductionSafetyPolicy{
		enabled:        true,
		isProduction:   detectProduction(),
		requireConfirm: true,
		blockedInProd: []string{
			"drop", "delete", "truncate", "reset", "restart", "kill", "stop", "disable",
		},
	}
}

func (p *ProductionSafetyPolicy) ID() string {
	return "production-safety"
}

func (p *ProductionSafetyPolicy) Name() string {
	return "Production Environment Safety"
}

func (p *ProductionSafetyPolicy) Description() string {
	return "Adds extra protection when running in production environments"
}

func (p *ProductionSafetyPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *ProductionSafetyPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
}

func (p *ProductionSafetyPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	if !p.isProduction {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     true,
			Severity:   guardrailstypes.SeverityInfo,
			Message:    "Not in production environment",
			Action:     guardrailstypes.ActionAllow,
			RiskLevel:  guardrailstypes.RiskLevelLow,
		}, nil
	}

	cmd := strings.ToLower(op.Command + " " + strings.Join(op.Args, " "))

	for _, blocked := range p.blockedInProd {
		if strings.Contains(cmd, blocked) {
			return &guardrailstypes.PolicyResult{
				PolicyID:   p.ID(),
				PolicyName: p.Name(),
				Passed:     false,
				Severity:   guardrailstypes.SeverityCritical,
				Message:    "Operation '" + blocked + "' requires explicit confirmation in production",
				Action:     guardrailstypes.ActionConfirm,
				RiskLevel:  guardrailstypes.RiskLevelCritical,
				Metadata: map[string]interface{}{
					"blocked_operation": blocked,
					"environment":       "production",
				},
			}, nil
		}
	}

	if p.isDatabaseOperation(cmd) {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     false,
			Severity:   guardrailstypes.SeverityWarning,
			Message:    "Database operation detected in production environment",
			Action:     guardrailstypes.ActionConfirm,
			RiskLevel:  guardrailstypes.RiskLevelHigh,
			Metadata: map[string]interface{}{
				"environment": "production",
				"operation":   "database",
			},
		}, nil
	}

	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     true,
		Severity:   guardrailstypes.SeverityInfo,
		Message:    "Production safety checks passed",
		Action:     guardrailstypes.ActionAllow,
		RiskLevel:  guardrailstypes.RiskLevelLow,
	}, nil
}

func (p *ProductionSafetyPolicy) isDatabaseOperation(cmd string) bool {
	dbOperations := []string{
		"db.", "database", "sql", "query", "migrate",
		"schema", "table", "insert", "update", "alter",
	}

	for _, op := range dbOperations {
		if strings.Contains(cmd, op) {
			return true
		}
	}
	return false
}

func (p *ProductionSafetyPolicy) SetProductionMode(isProd bool) {
	p.isProduction = isProd
}

func (p *ProductionSafetyPolicy) IsProduction() bool {
	return p.isProduction
}

func (p *ProductionSafetyPolicy) AddBlockedOperation(op string) {
	p.blockedInProd = append(p.blockedInProd, strings.ToLower(op))
}

func detectProduction() bool {
	prodVars := []string{"PRODUCTION", "PROD", "ENV", "NODE_ENV", "RAILS_ENV", "APP_ENV"}
	prodValues := []string{"production", "prod", "live", "production"}

	for _, v := range prodVars {
		val := os.Getenv(v)
		if val != "" {
			lowerVal := strings.ToLower(val)
			for _, prodVal := range prodValues {
				if lowerVal == prodVal {
					return true
				}
			}
		}
	}

	return false
}
