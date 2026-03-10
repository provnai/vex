package policy

import (
	"fmt"
	"os"
	"regexp"
	"strings"

	"gopkg.in/yaml.v3"
)

// PolicyAction represents what happens when a policy matches
type PolicyAction string

const (
	PolicyActionAllow PolicyAction = "allow"
	PolicyActionBlock PolicyAction = "block"
	PolicyActionWarn  PolicyAction = "warn"
	PolicyActionAudit PolicyAction = "audit"
)

// PolicySeverity represents the severity of a policy
type PolicySeverity string

const (
	SeverityInfo     PolicySeverity = "info"
	SeverityWarning  PolicySeverity = "warning"
	SeverityCritical PolicySeverity = "critical"
)

// Policy represents a safety policy
type Policy struct {
	ID          string          `yaml:"id"`
	Name        string          `yaml:"name"`
	Description string          `yaml:"description,omitempty"`
	Condition   PolicyCondition `yaml:"condition"`
	Action      PolicyAction    `yaml:"action"`
	Severity    PolicySeverity  `yaml:"severity,omitempty"`
	Enabled     bool            `yaml:"enabled,omitempty"`
	Tags        []string        `yaml:"tags,omitempty"`
}

// PolicyCondition defines when a policy matches
type PolicyCondition struct {
	ActionType     []string `yaml:"actionType,omitempty"`
	TargetMatch    string   `yaml:"targetMatch,omitempty"`
	TargetRegex    string   `yaml:"targetRegex,omitempty"`
	Classification []string `yaml:"classification,omitempty"`
	RiskLevel      string   `yaml:"riskLevel,omitempty"`
	Env            string   `yaml:"env,omitempty"`
	CustomExpr     string   `yaml:"customExpr,omitempty"`
}

// PolicyResult represents the result of evaluating a policy
type PolicyResult struct {
	PolicyID   string         `yaml:"policyId"`
	PolicyName string         `yaml:"policyName"`
	Matched    bool           `yaml:"matched"`
	Action     PolicyAction   `yaml:"action"`
	Severity   PolicySeverity `yaml:"severity"`
	Message    string         `yaml:"message,omitempty"`
}

// Evaluate checks if an action matches the policy condition
func (p *Policy) Evaluate(action ActionContext) *PolicyResult {
	result := &PolicyResult{
		PolicyID:   p.ID,
		PolicyName: p.Name,
		Matched:    false,
		Action:     PolicyActionAllow,
		Severity:   p.Severity,
	}

	// Check action type
	if len(p.Condition.ActionType) > 0 {
		found := false
		for _, t := range p.Condition.ActionType {
			if t == string(action.Type) {
				found = true
				break
			}
		}
		if !found {
			return result
		}
	}

	// Check target match (simple substring)
	if p.Condition.TargetMatch != "" {
		if !strings.Contains(action.Target, p.Condition.TargetMatch) {
			return result
		}
	}

	// Check target regex
	if p.Condition.TargetRegex != "" {
		re, err := regexp.Compile(p.Condition.TargetRegex)
		if err != nil {
			result.Message = fmt.Sprintf("invalid regex: %v", err)
			return result
		}
		if !re.MatchString(action.Target) {
			return result
		}
	}

	// Check classification
	if len(p.Condition.Classification) > 0 {
		found := false
		for _, c := range p.Condition.Classification {
			if c == action.Classification {
				found = true
				break
			}
		}
		if !found {
			return result
		}
	}

	// Policy matched
	result.Matched = true
	result.Action = p.Action
	result.Message = fmt.Sprintf("Policy '%s' matched", p.Name)

	return result
}

// ActionContext represents the context of an action being evaluated
type ActionContext struct {
	Type           string
	Target         string
	Classification string
	AgentID        string
	IntentID       string
	Environment    string
	RiskLevel      string
}

// PolicyEngine evaluates policies against actions
type PolicyEngine struct {
	policies map[string]*Policy
}

// NewPolicyEngine creates a new policy engine with default policies
func NewPolicyEngine() *PolicyEngine {
	engine := &PolicyEngine{
		policies: make(map[string]*Policy),
	}

	// Add default policies
	engine.AddDefaultPolicies()

	return engine
}

// AddDefaultPolicies adds built-in safety policies
func (e *PolicyEngine) AddDefaultPolicies() {
	// Prevent destructive operations without backup
	e.policies["prevent-destructive"] = &Policy{
		ID:          "prevent-destructive",
		Name:        "Prevent Destructive Operations",
		Description: "Block destructive operations without explicit confirmation",
		Condition: PolicyCondition{
			ActionType:     []string{"command", "database"},
			Classification: []string{"dangerous", "destructive"},
		},
		Action:   PolicyActionBlock,
		Severity: SeverityCritical,
		Enabled:  true,
	}

	// Production safety
	e.policies["production-safety"] = &Policy{
		ID:          "production-safety",
		Name:        "Production Safety",
		Description: "Extra checks for production environment",
		Condition: PolicyCondition{
			Env: "production",
		},
		Action:   PolicyActionWarn,
		Severity: SeverityWarning,
		Enabled:  true,
	}

	// Rate limiting
	e.policies["rate-limit"] = &Policy{
		ID:          "rate-limit",
		Name:        "Rate Limit",
		Description: "Limit API calls to prevent abuse",
		Condition: PolicyCondition{
			ActionType: []string{"api_call"},
		},
		Action:   PolicyActionBlock,
		Severity: SeverityWarning,
		Enabled:  true,
	}
}

// AddPolicy adds a policy to the engine
func (e *PolicyEngine) AddPolicy(policy *Policy) {
	policy.Enabled = true // Enable by default
	e.policies[policy.ID] = policy
}

// RemovePolicy removes a policy from the engine
func (e *PolicyEngine) RemovePolicy(id string) {
	delete(e.policies, id)
}

// GetPolicy returns a policy by ID
func (e *PolicyEngine) GetPolicy(id string) (*Policy, bool) {
	p, ok := e.policies[id]
	return p, ok
}

// ListPolicies returns all enabled policies
func (e *PolicyEngine) ListPolicies() []*Policy {
	policies := make([]*Policy, 0, len(e.policies))
	for _, p := range e.policies {
		if p.Enabled {
			policies = append(policies, p)
		}
	}
	return policies
}

// Evaluate evaluates all policies against an action
func (e *PolicyEngine) Evaluate(action ActionContext) []*PolicyResult {
	results := make([]*PolicyResult, 0)

	for _, policy := range e.policies {
		if !policy.Enabled {
			continue
		}
		result := policy.Evaluate(action)
		if result.Matched {
			results = append(results, result)
		}
	}

	return results
}

// ShouldAllow determines if an action should be allowed
func (e *PolicyEngine) ShouldAllow(action ActionContext) (bool, []*PolicyResult) {
	results := e.Evaluate(action)

	for _, r := range results {
		if r.Matched && r.Action == PolicyActionBlock {
			return false, results
		}
	}

	return true, results
}

// LoadPoliciesFromDir loads all YAML policies from a directory
func LoadPoliciesFromDir(dir string) ([]*Policy, error) {
	var policies []*Policy

	entries, err := os.ReadDir(dir)
	if err != nil {
		return nil, fmt.Errorf("failed to read policy directory: %w", err)
	}

	for _, entry := range entries {
		if entry.IsDir() || !strings.HasSuffix(entry.Name(), ".yaml") {
			continue
		}

		path := dir + "/" + entry.Name()
		policy, err := LoadPolicyFromFile(path)
		if err != nil {
			return nil, fmt.Errorf("failed to load policy %s: %w", entry.Name(), err)
		}
		policies = append(policies, policy)
	}

	return policies, nil
}

// LoadPolicyFromFile loads a policy from a YAML file
func LoadPolicyFromFile(path string) (*Policy, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read policy file: %w", err)
	}

	var policy Policy
	if err := parseYAML(data, &policy); err != nil {
		return nil, fmt.Errorf("failed to parse policy: %w", err)
	}

	return &policy, nil
}

// parseYAML parses YAML data
func parseYAML(data []byte, v interface{}) error {
	return yaml.Unmarshal(data, v)
}

// PolicyStore provides storage for policies
type PolicyStore struct {
	engine *PolicyEngine
}

// NewPolicyStore creates a new policy store
func NewPolicyStore() *PolicyStore {
	return &PolicyStore{
		engine: NewPolicyEngine(),
	}
}

// Save saves a policy
func (s *PolicyStore) Save(policy *Policy) error {
	s.engine.AddPolicy(policy)
	return nil
}

// List returns all policies
func (s *PolicyStore) List() []*Policy {
	return s.engine.ListPolicies()
}

// Delete removes a policy
func (s *PolicyStore) Delete(id string) error {
	s.engine.RemovePolicy(id)
	return nil
}
