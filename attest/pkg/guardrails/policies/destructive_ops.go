package policies

import (
	"context"
	"regexp"
	"strings"

	"github.com/provnai/attest/pkg/guardrailstypes"
)

type DestructiveOpsPolicy struct {
	enabled       bool
	blockedCmds   []string
	blockedArgs   []string
	allowOverride bool
}

func NewDestructiveOpsPolicy() *DestructiveOpsPolicy {
	return &DestructiveOpsPolicy{
		enabled: true,
		blockedCmds: []string{
			"rm", "del", "rmdir", "format", "mkfs", "dd", "shred", "wipe", ">",
		},
		blockedArgs: []string{
			"-rf", "-fr", "--force", "/", "/*", "C:\\", "/dev/", "format", "DROP", "DELETE",
		},
		allowOverride: true,
	}
}

func (p *DestructiveOpsPolicy) ID() string {
	return "prevent-destructive"
}

func (p *DestructiveOpsPolicy) Name() string {
	return "Destructive Operations Blocker"
}

func (p *DestructiveOpsPolicy) Description() string {
	return "Prevents execution of destructive commands like rm -rf, format, etc."
}

func (p *DestructiveOpsPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *DestructiveOpsPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
}

func (p *DestructiveOpsPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	cmd := strings.ToLower(op.Command)
	fullCmd := strings.ToLower(cmd + " " + strings.Join(op.Args, " "))

	for _, blocked := range p.blockedCmds {
		if strings.HasPrefix(cmd, blocked) || strings.Contains(fullCmd, blocked) {
			return &guardrailstypes.PolicyResult{
				PolicyID:   p.ID(),
				PolicyName: p.Name(),
				Passed:     false,
				Severity:   guardrailstypes.SeverityBlocked,
				Message:    "Potentially destructive command detected: " + blocked,
				Action:     p.getAction(),
				RiskLevel:  guardrailstypes.RiskLevelCritical,
			}, nil
		}
	}

	for _, blocked := range p.blockedArgs {
		if strings.Contains(fullCmd, blocked) {
			return &guardrailstypes.PolicyResult{
				PolicyID:   p.ID(),
				PolicyName: p.Name(),
				Passed:     false,
				Severity:   guardrailstypes.SeverityBlocked,
				Message:    "Destructive arguments detected: " + blocked,
				Action:     p.getAction(),
				RiskLevel:  guardrailstypes.RiskLevelCritical,
			}, nil
		}
	}

	sqlPattern := regexp.MustCompile(`(?i)(DROP\s+TABLE|DELETE\s+FROM)\s+`)
	if sqlPattern.MatchString(fullCmd) && !strings.Contains(fullCmd, "where") {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     false,
			Severity:   guardrailstypes.SeverityCritical,
			Message:    "SQL destructive operation without WHERE clause detected",
			Action:     p.getAction(),
			RiskLevel:  guardrailstypes.RiskLevelCritical,
		}, nil
	}

	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     true,
		Severity:   guardrailstypes.SeverityInfo,
		Message:    "No destructive operations detected",
		Action:     guardrailstypes.ActionAllow,
		RiskLevel:  guardrailstypes.RiskLevelLow,
	}, nil
}

func (p *DestructiveOpsPolicy) getAction() guardrailstypes.Action {
	if p.allowOverride {
		return guardrailstypes.ActionConfirm
	}
	return guardrailstypes.ActionBlock
}

func (p *DestructiveOpsPolicy) SetAllowOverride(allow bool) {
	p.allowOverride = allow
}

func (p *DestructiveOpsPolicy) AddBlockedCommand(cmd string) {
	p.blockedCmds = append(p.blockedCmds, strings.ToLower(cmd))
}

func (p *DestructiveOpsPolicy) AddBlockedArg(arg string) {
	p.blockedArgs = append(p.blockedArgs, strings.ToLower(arg))
}
