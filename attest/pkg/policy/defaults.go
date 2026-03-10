package policy

import (
	"fmt"
	"os"
	"os/exec"
	"regexp"
	"time"
)

// DefaultPolicyDir is the default directory for policies
const DefaultPolicyDir = ".attest/policies"

// CreateDefaultPolicies creates all default safety policies
func CreateDefaultPolicies() []*Policy {
	return []*Policy{
		{
			ID:          "prevent-destructive",
			Name:        "Prevent Destructive Operations",
			Description: "Block destructive operations without explicit confirmation",
			Condition: PolicyCondition{
				ActionType:     []string{"command", "database"},
				Classification: []string{"dangerous", "destructive"},
				TargetRegex:    `(?i)(rm|del|format|mkfs|fdisk|dd)\s*(-rf|/r|/s|/q|.*--force)`,
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
			Tags:     []string{"safety", "critical"},
		},
		{
			ID:          "prevent-privilege-escalation",
			Name:        "Prevent Privilege Escalation",
			Description: "Block commands attempting to gain root/admin privileges",
			Condition: PolicyCondition{
				ActionType:  []string{"command"},
				TargetRegex: `(?i)(sudo|su\s|chmod\s+777|chown\s+root.*admin)`,
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
			Tags:     []string{"security", "critical"},
		},
		{
			ID:          "production-safety",
			Name:        "Production Safety",
			Description: "Extra checks for production environment",
			Condition: PolicyCondition{
				Env: "production",
			},
			Action:   PolicyActionWarn,
			Severity: SeverityWarning,
			Enabled:  true,
			Tags:     []string{"production", "safety"},
		},
		{
			ID:          "rate-limit-api",
			Name:        "Rate Limit API Calls",
			Description: "Limit API calls to prevent abuse",
			Condition: PolicyCondition{
				ActionType: []string{"api_call"},
			},
			Action:   PolicyActionBlock,
			Severity: SeverityWarning,
			Enabled:  true,
			Tags:     []string{"api", "rate-limiting"},
		},
		{
			ID:          "prevent-network-damage",
			Name:        "Prevent Network Damage",
			Description: "Block commands that could damage network configuration",
			Condition: PolicyCondition{
				ActionType:  []string{"command"},
				TargetRegex: `(?i)(iptables|ufw|ifconfig|route|netsh|firewall).*(delete|remove|flush|reset)`,
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
			Tags:     []string{"network", "safety"},
		},
		{
			ID:          "require-commit-message",
			Name:        "Require Commit Message",
			Description: "Ensure Git commits have meaningful messages",
			Condition: PolicyCondition{
				ActionType:  []string{"git"},
				TargetMatch: "commit",
			},
			Action:   PolicyActionWarn,
			Severity: SeverityInfo,
			Enabled:  true,
			Tags:     []string{"git", "quality"},
		},
		{
			ID:          "database-readonly",
			Name:        "Database Read-Only Preference",
			Description: "Prefer read operations on databases",
			Condition: PolicyCondition{
				ActionType:     []string{"database"},
				TargetRegex:    `(?i)(SELECT|SHOW|DESCRIBE|EXPLAIN)`,
				Classification: []string{"read"},
			},
			Action:   PolicyActionAllow,
			Severity: SeverityInfo,
			Enabled:  true,
			Tags:     []string{"database", "safety"},
		},
		{
			ID:          "prevent-database-destructive",
			Name:        "Prevent Destructive Database Operations",
			Description: "Block DROP, TRUNCATE, or DELETE without conditions",
			Condition: PolicyCondition{
				ActionType:  []string{"database"},
				TargetRegex: `(?i)(DROP\s+(TABLE|DATABASE)|TRUNCATE|DELETE\s+FROM\s+(?!.*WHERE))`,
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
			Tags:     []string{"database", "critical"},
		},
		{
			ID:          "shell-injection-prevention",
			Name:        "Prevent Shell Injection",
			Description: "Block commands with potential shell injection",
			Condition: PolicyCondition{
				ActionType:  []string{"command"},
				TargetRegex: `(?i)(;|\||&&|\$\(.*\)|` + "`" + `.*` + "`" + `|\$\{.*\})`,
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
			Tags:     []string{"security", "injection"},
		},
		{
			ID:          "file-permission-safety",
			Name:        "File Permission Safety",
			Description: "Warn on overly permissive file permissions",
			Condition: PolicyCondition{
				ActionType:  []string{"command"},
				TargetRegex: `(?i)(chmod\s+777|chmod\s+755.*\+\s*w|umask\s+000)`,
			},
			Action:   PolicyActionWarn,
			Severity: SeverityWarning,
			Enabled:  true,
			Tags:     []string{"security", "permissions"},
		},
	}
}

// CreateDefaultPolicyFiles creates policy YAML files in the specified directory
func CreateDefaultPolicyFiles(dir string) error {
	if err := os.MkdirAll(dir, 0755); err != nil {
		return fmt.Errorf("failed to create policy directory: %w", err)
	}

	policies := CreateDefaultPolicies()
	for _, p := range policies {
		path := dir + "/" + p.ID + ".yaml"
		content := ExportToYAML(p)
		if err := os.WriteFile(path, []byte(content), 0644); err != nil {
			return fmt.Errorf("failed to write policy %s: %w", p.ID, err)
		}
	}

	return nil
}

// LoadPolicyFromProcess runs a command and captures its output
func LoadPolicyFromProcess(name string, args ...string) (*Policy, error) {
	cmd := exec.Command(name, args...)
	output, err := cmd.CombinedOutput()
	if err != nil {
		return nil, fmt.Errorf("command failed: %w, output: %s", err, output)
	}
	return LoadPolicyFromBytes(output)
}

// ValidateTargetRegex validates a target regex pattern
func ValidateTargetRegex(pattern string) error {
	_, err := regexp.Compile(pattern)
	return err
}

// GetPolicyRecommendations returns policy recommendations based on environment
func GetPolicyRecommendations(env string) []*Policy {
	var recommendations []*Policy

	if env == "production" {
		recommendations = append(recommendations, &Policy{
			ID:          "production-recommendation-1",
			Name:        "Production Mode",
			Description: "Enable all safety checks for production",
			Condition: PolicyCondition{
				Env: "production",
			},
			Action:   PolicyActionBlock,
			Severity: SeverityCritical,
			Enabled:  true,
		})
	}

	return recommendations
}

// PolicyBundle represents a collection of related policies
type PolicyBundle struct {
	Name        string
	Description string
	Policies    []*Policy
	Version     string
	CreatedAt   string
}

// CreateSecurityBundle creates a security-focused policy bundle
func CreateSecurityBundle() *PolicyBundle {
	return &PolicyBundle{
		Name:        "Security Essentials",
		Description: "Core security policies for safe AI agent operation",
		Version:     "1.0.0",
		CreatedAt:   time.Now().Format("2006-01-02"),
		Policies: []*Policy{
			{
				ID:        "security-block-shell",
				Name:      "Block Shell Injection",
				Condition: PolicyCondition{TargetRegex: `(?i)(;|\||&&|\$\(.*\)|` + "`" + `.*` + "`" + `)`},
				Action:    PolicyActionBlock,
				Severity:  SeverityCritical,
				Enabled:   true,
			},
			{
				ID:        "security-block-sudo",
				Name:      "Block Sudo Escalation",
				Condition: PolicyCondition{TargetRegex: `(?i)^(sudo|su\s)`},
				Action:    PolicyActionBlock,
				Severity:  SeverityCritical,
				Enabled:   true,
			},
			{
				ID:        "security-block-chmod-777",
				Name:      "Block Chmod 777",
				Condition: PolicyCondition{TargetRegex: `(?i)chmod\s+777`},
				Action:    PolicyActionBlock,
				Severity:  SeverityWarning,
				Enabled:   true,
			},
		},
	}
}

// CreateDevelopmentBundle creates a development-focused policy bundle
func CreateDevelopmentBundle() *PolicyBundle {
	return &PolicyBundle{
		Name:        "Development Essentials",
		Description: "Policies optimized for development workflows",
		Version:     "1.0.0",
		CreatedAt:   time.Now().Format("2006-01-02"),
		Policies: []*Policy{
			{
				ID:        "dev-warn-on-cleanup",
				Name:      "Warn on Cleanup Scripts",
				Condition: PolicyCondition{TargetMatch: "cleanup"},
				Action:    PolicyActionWarn,
				Severity:  SeverityWarning,
				Enabled:   true,
			},
			{
				ID:        "dev-allow-debug",
				Name:      "Allow Debug Commands",
				Condition: PolicyCondition{TargetMatch: "debug"},
				Action:    PolicyActionAllow,
				Severity:  SeverityInfo,
				Enabled:   true,
			},
		},
	}
}
