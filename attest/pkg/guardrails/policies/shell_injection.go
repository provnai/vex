package policies

import (
	"context"
	"regexp"
	"strings"

	"github.com/provnai/attest/pkg/guardrailstypes"
)

type ShellInjectionPolicy struct {
	enabled           bool
	dangerousPatterns []*regexp.Regexp
	blockedChars      []string
}

func NewShellInjectionPolicy() *ShellInjectionPolicy {
	p := &ShellInjectionPolicy{
		enabled: true,
		blockedChars: []string{
			";", "|", "&", "$", "`", "\\",
		},
	}

	p.dangerousPatterns = []*regexp.Regexp{
		regexp.MustCompile(`\$\([^)]+\)`),
		regexp.MustCompile("`[^`]+`"),
		regexp.MustCompile(`;\s*\w+`),
		regexp.MustCompile(`\|\s*\w+`),
		regexp.MustCompile(`&&\s*\w+`),
		regexp.MustCompile(`\|\|\s*\w+`),
		regexp.MustCompile(`>\s*\w+`),
		regexp.MustCompile(`<\s*\w+`),
		regexp.MustCompile(`\$\w+`),
		regexp.MustCompile(`\$\{[^}]+\}`),
		regexp.MustCompile(`\.\.\/`),
		regexp.MustCompile(`\.\.\\`),
		regexp.MustCompile(`\x00`),
	}

	return p
}

func (p *ShellInjectionPolicy) ID() string {
	return "prevent-shell-injection"
}

func (p *ShellInjectionPolicy) Name() string {
	return "Shell Injection Prevention"
}

func (p *ShellInjectionPolicy) Description() string {
	return "Blocks commands containing potential shell injection patterns"
}

func (p *ShellInjectionPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *ShellInjectionPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
}

func (p *ShellInjectionPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	fullCmd := op.Command + " " + strings.Join(op.Args, " ")

	for _, pattern := range p.dangerousPatterns {
		if matches := pattern.FindString(fullCmd); matches != "" {
			return &guardrailstypes.PolicyResult{
				PolicyID:   p.ID(),
				PolicyName: p.Name(),
				Passed:     false,
				Severity:   guardrailstypes.SeverityBlocked,
				Message:    "Potential shell injection detected: " + sanitizeForDisplay(matches),
				Action:     guardrailstypes.ActionBlock,
				RiskLevel:  guardrailstypes.RiskLevelCritical,
				Metadata: map[string]interface{}{
					"pattern":  matches,
					"full_cmd": sanitizeForDisplay(fullCmd),
				},
			}, nil
		}
	}

	for _, char := range p.blockedChars {
		if strings.Contains(fullCmd, char) {
			if char == "$" && !p.isVariableExpansion(fullCmd) {
				continue
			}

			return &guardrailstypes.PolicyResult{
				PolicyID:   p.ID(),
				PolicyName: p.Name(),
				Passed:     false,
				Severity:   guardrailstypes.SeverityBlocked,
				Message:    "Blocked character detected: '" + char + "'",
				Action:     guardrailstypes.ActionBlock,
				RiskLevel:  guardrailstypes.RiskLevelCritical,
				Metadata: map[string]interface{}{
					"character": char,
					"position":  strings.Index(fullCmd, char),
				},
			}, nil
		}
	}

	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     true,
		Severity:   guardrailstypes.SeverityInfo,
		Message:    "No shell injection patterns detected",
		Action:     guardrailstypes.ActionAllow,
		RiskLevel:  guardrailstypes.RiskLevelLow,
	}, nil
}

func (p *ShellInjectionPolicy) isVariableExpansion(cmd string) bool {
	varPattern := regexp.MustCompile(`\$\w|\$\{`)
	return varPattern.MatchString(cmd)
}

func (p *ShellInjectionPolicy) AddPattern(pattern string) error {
	re, err := regexp.Compile(pattern)
	if err != nil {
		return err
	}
	p.dangerousPatterns = append(p.dangerousPatterns, re)
	return nil
}

func (p *ShellInjectionPolicy) AddBlockedChar(char string) {
	p.blockedChars = append(p.blockedChars, char)
}

func sanitizeForDisplay(s string) string {
	s = strings.ReplaceAll(s, "\x00", "[NULL]")
	s = strings.ReplaceAll(s, "\n", "[NL]")
	s = strings.ReplaceAll(s, "\r", "[CR]")

	if len(s) > 50 {
		s = s[:47] + "..."
	}

	return s
}
