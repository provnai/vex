package policies

import (
	"context"
	"fmt"
	"regexp"
	"strconv"
	"strings"

	"github.com/provnai/attest/pkg/guardrailstypes"
)

type FileLimitPolicy struct {
	enabled      bool
	maxFileCount int
	maxFileSize  int64
}

func NewFileLimitPolicy(maxFiles int) *FileLimitPolicy {
	return &FileLimitPolicy{
		enabled:      true,
		maxFileCount: maxFiles,
		maxFileSize:  100 * 1024 * 1024,
	}
}

func (p *FileLimitPolicy) ID() string {
	return "file-limit"
}

func (p *FileLimitPolicy) Name() string {
	return "File Operation Limiter"
}

func (p *FileLimitPolicy) Description() string {
	return fmt.Sprintf("Blocks operations affecting more than %d files", p.maxFileCount)
}

func (p *FileLimitPolicy) IsEnabled() bool {
	return p.enabled
}

func (p *FileLimitPolicy) SetEnabled(enabled bool) {
	p.enabled = enabled
}

func (p *FileLimitPolicy) Evaluate(ctx context.Context, op *guardrailstypes.Operation) (*guardrailstypes.PolicyResult, error) {
	if !p.isFileOperation(op) {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     true,
			Severity:   guardrailstypes.SeverityInfo,
			Message:    "Not a file operation",
			Action:     guardrailstypes.ActionAllow,
			RiskLevel:  guardrailstypes.RiskLevelLow,
		}, nil
	}

	fileCount := p.estimateFileCount(op)

	if fileCount > p.maxFileCount {
		return &guardrailstypes.PolicyResult{
			PolicyID:   p.ID(),
			PolicyName: p.Name(),
			Passed:     false,
			Severity:   guardrailstypes.SeverityCritical,
			Message:    fmt.Sprintf("Operation would affect %d files, exceeding limit of %d", fileCount, p.maxFileCount),
			Action:     guardrailstypes.ActionConfirm,
			RiskLevel:  guardrailstypes.RiskLevelHigh,
			Metadata: map[string]interface{}{
				"estimated_files": fileCount,
				"file_limit":      p.maxFileCount,
			},
		}, nil
	}

	return &guardrailstypes.PolicyResult{
		PolicyID:   p.ID(),
		PolicyName: p.Name(),
		Passed:     true,
		Severity:   guardrailstypes.SeverityInfo,
		Message:    fmt.Sprintf("Operation affects %d files (within limit)", fileCount),
		Action:     guardrailstypes.ActionAllow,
		RiskLevel:  guardrailstypes.RiskLevelLow,
		Metadata: map[string]interface{}{
			"estimated_files": fileCount,
		},
	}, nil
}

func (p *FileLimitPolicy) isFileOperation(op *guardrailstypes.Operation) bool {
	cmd := strings.ToLower(op.Command)

	fileCmds := []string{
		"rm", "del", "delete", "cp", "copy", "mv", "move", "find",
		"ls", "dir", "chmod", "chown", "tar", "zip", "unzip", "rsync",
	}

	for _, fileCmd := range fileCmds {
		if cmd == fileCmd || strings.HasPrefix(cmd, fileCmd) {
			return true
		}
	}

	return false
}

func (p *FileLimitPolicy) estimateFileCount(op *guardrailstypes.Operation) int {
	fullCmd := op.Command + " " + strings.Join(op.Args, " ")
	cmd := strings.ToLower(fullCmd)

	count := 1

	if strings.Contains(cmd, "-r") || strings.Contains(cmd, "-rf") ||
		strings.Contains(cmd, "--recursive") || strings.Contains(cmd, "**/*") {
		count = 100
	}

	globPatterns := []string{"*", "?", "[", "]"}
	for _, pattern := range globPatterns {
		if strings.Contains(cmd, pattern) {
			count = max(count, 50)
		}
	}

	fileCount := p.countFileArguments(op.Args)
	count = max(count, fileCount)

	numberRegex := regexp.MustCompile(`\b(\d+)\b`)
	matches := numberRegex.FindAllString(cmd, -1)
	for _, match := range matches {
		if num, err := strconv.Atoi(match); err == nil && num > 0 && num < 10000 {
			count = max(count, num)
		}
	}

	return count
}

func (p *FileLimitPolicy) countFileArguments(args []string) int {
	count := 0
	for _, arg := range args {
		if strings.HasPrefix(arg, "-") {
			continue
		}
		count++
	}
	return count
}

func (p *FileLimitPolicy) SetMaxFileCount(maxFiles int) {
	p.maxFileCount = maxFiles
}

func (p *FileLimitPolicy) GetMaxFileCount() int {
	return p.maxFileCount
}

func max(a, b int) int {
	if b > a {
		return b
	}
	return a
}
