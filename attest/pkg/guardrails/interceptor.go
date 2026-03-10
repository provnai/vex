package guardrails

import (
	"context"
	"fmt"
	"os"
	"os/exec"
	"strings"
	"time"

	"github.com/fatih/color"
)

// Interceptor handles command interception and policy enforcement
type Interceptor struct {
	policies    []Policy
	manager     *CheckpointManager
	enabled     bool
	interactive bool
}

// NewInterceptor creates a new guardrail interceptor
func NewInterceptor(manager *CheckpointManager, interactive bool) *Interceptor {
	return &Interceptor{
		policies:    []Policy{},
		manager:     manager,
		enabled:     true,
		interactive: interactive,
	}
}

// AddPolicy registers a policy with the interceptor
func (i *Interceptor) AddPolicy(policy Policy) {
	i.policies = append(i.policies, policy)
}

// SetEnabled enables or disables the interceptor
func (i *Interceptor) SetEnabled(enabled bool) {
	i.enabled = enabled
}

// IsEnabled returns whether the interceptor is enabled
func (i *Interceptor) IsEnabled() bool {
	return i.enabled
}

// GetPolicies returns all registered policies
func (i *Interceptor) GetPolicies() []Policy {
	return i.policies
}

// ExecuteWithGuardrails runs a command with full guardrail protection
func (i *Interceptor) ExecuteWithGuardrails(ctx context.Context, command string, args []string) (*ExecutionResult, error) {
	if !i.enabled {
		return i.executeDirect(ctx, command, args)
	}

	// Create operation representation
	op := &Operation{
		ID:         fmt.Sprintf("op_%d", time.Now().UnixNano()),
		Type:       "shell",
		Command:    command,
		Args:       args,
		Env:        getEnvMap(),
		WorkingDir: getWorkingDir(),
	}

	// Evaluate all policies
	results, riskLevel := i.evaluatePolicies(ctx, op)

	// Check if any policy blocked the operation
	for _, result := range results {
		if result.Action == ActionBlock {
			return nil, &GuardrailViolationError{
				PolicyID:   result.PolicyID,
				PolicyName: result.PolicyName,
				Operation:  fmt.Sprintf("%s %s", command, strings.Join(args, " ")),
				Message:    result.Message,
				Severity:   result.Severity,
			}
		}
	}

	// Display warnings for violated policies
	for _, result := range results {
		if !result.Passed && result.Action == ActionWarn {
			displayWarning(result)
		}
	}

	// If risky, create checkpoint
	var checkpoint *Checkpoint
	if riskLevel != RiskLevelLow {
		fmt.Println()
		yellow := color.New(color.FgYellow).SprintFunc()
		fmt.Printf("%s Creating checkpoint for risky operation...\n", yellow("⚠"))

		var err error
		checkpoint, err = i.manager.CreateCheckpoint(ctx, op)
		if err != nil {
			// If checkpoint fails, we might want to block depending on settings
			fmt.Printf("   ✗ Failed to create checkpoint: %v\n", err)
			if riskLevel == RiskLevelCritical {
				return nil, fmt.Errorf("critical operation without checkpoint: %w", err)
			}
		} else {
			green := color.New(color.FgGreen).SprintFunc()
			fmt.Printf("   %s Checkpoint created: %s\n", green("✓"), checkpoint.ID)
		}
	}

	// Handle confirmation requirements
	for _, result := range results {
		if result.Action == ActionConfirm {
			if !i.confirmOperation(result, op) {
				return &ExecutionResult{
					Blocked:    true,
					BlockedBy:  result.PolicyName,
					Checkpoint: checkpoint,
				}, nil
			}
		}
	}

	// Execute the command
	result := i.executeCommand(ctx, command, args)
	result.Checkpoint = checkpoint
	result.PolicyResults = results

	// If execution failed and we have a checkpoint, offer rollback
	if !result.Success && checkpoint != nil {
		fmt.Println()
		red := color.New(color.FgRed).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()
		fmt.Printf("%s Command failed with exit code %d\n", red("❌"), result.ExitCode)
		fmt.Printf("%s Auto-rolling back to checkpoint %s\n", yellow("⟲"), checkpoint.ID)

		rollbackResult, err := i.manager.Rollback(ctx, checkpoint.ID)
		if err != nil {
			fmt.Printf("   ✗ Rollback failed: %v\n", err)
			result.RollbackError = err
		} else {
			green := color.New(color.FgGreen).SprintFunc()
			fmt.Printf("   %s Rollback complete - system restored\n", green("✓"))
			result.RollbackResult = rollbackResult
		}
	}

	return result, nil
}

// ExecutionResult represents the outcome of command execution
type ExecutionResult struct {
	Success        bool
	ExitCode       int
	Stdout         string
	Stderr         string
	Duration       time.Duration
	Blocked        bool
	BlockedBy      string
	Checkpoint     *Checkpoint
	PolicyResults  []*PolicyResult
	RollbackResult *RollbackResult
	RollbackError  error
}

// evaluatePolicies checks all registered policies against the operation
func (i *Interceptor) evaluatePolicies(ctx context.Context, op *Operation) ([]*PolicyResult, RiskLevel) {
	var results []*PolicyResult
	highestRisk := RiskLevelLow

	for _, policy := range i.policies {
		if !policy.IsEnabled() {
			continue
		}

		result, err := policy.Evaluate(ctx, op)
		if err != nil {
			// If policy evaluation fails, treat as warning
			result = &PolicyResult{
				PolicyID:   policy.ID(),
				PolicyName: policy.Name(),
				Passed:     false,
				Severity:   SeverityWarning,
				Message:    fmt.Sprintf("Policy evaluation error: %v", err),
				Action:     ActionWarn,
			}
		}

		results = append(results, result)

		// Update highest risk level
		if !result.Passed && result.RiskLevel != "" {
			highestRisk = maxRisk(highestRisk, result.RiskLevel)
		}
	}

	return results, highestRisk
}

// executeCommand runs the actual command
func (i *Interceptor) executeCommand(ctx context.Context, command string, args []string) *ExecutionResult {
	start := time.Now()

	cmd := exec.CommandContext(ctx, command, args...)
	cmd.Dir = getWorkingDir()
	cmd.Env = os.Environ()

	output, err := cmd.CombinedOutput()

	result := &ExecutionResult{
		Duration: time.Since(start),
		Stdout:   string(output),
	}

	if err != nil {
		if exitErr, ok := err.(*exec.ExitError); ok {
			result.ExitCode = exitErr.ExitCode()
		} else {
			result.ExitCode = -1
		}
		result.Success = false
		result.Stderr = err.Error()
	} else {
		result.Success = true
		result.ExitCode = 0
	}

	return result
}

// executeDirect runs a command without any guardrails
func (i *Interceptor) executeDirect(ctx context.Context, command string, args []string) (*ExecutionResult, error) {
	result := i.executeCommand(ctx, command, args)
	return result, nil
}

// confirmOperation prompts user for confirmation
func (i *Interceptor) confirmOperation(result *PolicyResult, op *Operation) bool {
	if !i.interactive {
		return false
	}

	yellow := color.New(color.FgYellow).SprintFunc()
	_ = yellow
	red := color.New(color.FgRed, color.Bold).SprintFunc()

	fmt.Println()
	fmt.Printf("%s DANGER: %s\n", red("⚠"), result.Message)
	fmt.Printf("   Operation: %s %s\n", op.Command, strings.Join(op.Args, " "))
	fmt.Printf("   Policy: %s\n", result.PolicyName)
	fmt.Println()
	fmt.Printf("Type %s to confirm: ", red("'DESTROY'"))

	var input string
	if _, err := fmt.Scanln(&input); err != nil {
		return false
	}

	return input == "DESTROY"
}

// displayWarning shows a warning message for policy violations
func displayWarning(result *PolicyResult) {
	yellow := color.New(color.FgYellow).SprintFunc()
	fmt.Printf("%s Warning [%s]: %s\n", yellow("⚠"), result.PolicyName, result.Message)
}

// YellowFunc returns a yellow color function for external use
func YellowFunc() func(...interface{}) string {
	return color.New(color.FgYellow).SprintFunc()
}

// maxRisk returns the higher of two risk levels
func maxRisk(a, b RiskLevel) RiskLevel {
	order := map[RiskLevel]int{
		RiskLevelLow:      0,
		RiskLevelMedium:   1,
		RiskLevelHigh:     2,
		RiskLevelCritical: 3,
	}

	if order[b] > order[a] {
		return b
	}
	return a
}

// getEnvMap returns environment variables as a map
func getEnvMap() map[string]string {
	env := make(map[string]string)
	for _, e := range os.Environ() {
		if i := strings.Index(e, "="); i > 0 {
			env[e[:i]] = e[i+1:]
		}
	}
	return env
}

// getWorkingDir returns the current working directory
func getWorkingDir() string {
	dir, _ := os.Getwd()
	return dir
}
