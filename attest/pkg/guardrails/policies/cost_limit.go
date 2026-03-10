package policies

import (
	"context"
	"fmt"
	"strings"

	"github.com/provnai/attest/pkg/cost"
	"github.com/provnai/attest/pkg/guardrailstypes"
)

type CostLimitPolicy struct {
	enabled      bool
	maxCost      float64
	modelPricing map[string]cost.ModelPricing
}

func NewCostLimitPolicy(maxCost float64) *CostLimitPolicy {
	return &CostLimitPolicy{
		enabled:      true,
		maxCost:      maxCost,
		modelPricing: cost.KnownModelPricing,
	}
}

func (p *CostLimitPolicy) ID() string {
	return "cost-limit"
}

func (p *CostLimitPolicy) Name() string {
	return "API Cost Limiter"
}

func (p *CostLimitPolicy) Description() string {
	return fmt.Sprintf("Blocks API calls estimated to cost more than $%.2f", p.maxCost)
}

func (p *CostLimitPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *CostLimitPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
}

func (p *CostLimitPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	if !p.isAPICall(op) {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     true,
			Severity:   guardrailstypes.SeverityInfo,
			Message:    "Not an API call",
			Action:     guardrailstypes.ActionAllow,
			RiskLevel:  guardrailstypes.RiskLevelLow,
		}, nil
	}

	estimatedCost := p.estimateCost(op)

	if estimatedCost > p.maxCost {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     false,
			Severity:   guardrailstypes.SeverityCritical,
			Message:    fmt.Sprintf("Estimated cost $%.2f exceeds limit of $%.2f", estimatedCost, p.maxCost),
			Action:     guardrailstypes.ActionBlock,
			RiskLevel:  guardrailstypes.RiskLevelHigh,
			Metadata: map[string]interface{}{
				"estimated_cost": estimatedCost,
				"cost_limit":     p.maxCost,
			},
		}, nil
	}

	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     true,
		Severity:   guardrailstypes.SeverityInfo,
		Message:    fmt.Sprintf("Estimated cost $%.2f within limits", estimatedCost),
		Action:     guardrailstypes.ActionAllow,
		RiskLevel:  guardrailstypes.RiskLevelLow,
		Metadata: map[string]interface{}{
			"estimated_cost": estimatedCost,
		},
	}, nil
}

func (p *CostLimitPolicy) isAPICall(op *guardrailstypes.Operation) bool {
	cmd := strings.ToLower(op.Command)

	apiCmds := []string{"curl", "wget", "http", "api", "openai", "anthropic", "claude", "gpt"}
	for _, apiCmd := range apiCmds {
		if strings.Contains(cmd, apiCmd) {
			return true
		}
	}

	for _, arg := range op.Args {
		lowerArg := strings.ToLower(arg)
		if strings.Contains(lowerArg, "api.") ||
			strings.Contains(lowerArg, "openai.com") ||
			strings.Contains(lowerArg, "anthropic.com") ||
			strings.Contains(lowerArg, "apikey") ||
			strings.Contains(lowerArg, "token") {
			return true
		}
	}

	return false
}

func (p *CostLimitPolicy) estimateCost(op *guardrailstypes.Operation) float64 {
	cmd := strings.ToLower(op.Command + " " + strings.Join(op.Args, " "))

	estimatedCost := 0.10

	for modelID, pricing := range p.modelPricing {
		if strings.Contains(cmd, strings.ToLower(modelID)) {
			estimatedInputTokens := int64(4000)
			estimatedOutputTokens := int64(1000)

			inputCost := (float64(estimatedInputTokens) / 1000.0) * pricing.InputPrice
			outputCost := (float64(estimatedOutputTokens) / 1000.0) * pricing.OutputPrice

			return inputCost + outputCost
		}
	}

	return estimatedCost
}

func (p *CostLimitPolicy) SetMaxCost(maxCost float64) {
	p.maxCost = maxCost
}

func (p *CostLimitPolicy) GetMaxCost() float64 {
	return p.maxCost
}
