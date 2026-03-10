package guardrails

import (
	"context"
	"encoding/json"
	"fmt"
	"os"
	"path/filepath"

	"github.com/provnai/attest/pkg/guardrails/policies"
	"github.com/provnai/attest/pkg/policy"
)

// PolicyRegistry holds all available policies
type PolicyRegistry struct {
	policies map[string]Policy
}

// NewPolicyRegistry creates a new policy registry with built-in policies
func NewPolicyRegistry() *PolicyRegistry {
	registry := &PolicyRegistry{
		policies: make(map[string]Policy),
	}

	// Register built-in policies
	registry.Register(policies.NewDestructiveOpsPolicy())
	registry.Register(policies.NewCostLimitPolicy(5.0)) // $5 default limit
	registry.Register(policies.NewFileLimitPolicy(10))  // 10 files default
	registry.Register(policies.NewProductionSafetyPolicy())
	registry.Register(policies.NewShellInjectionPolicy())

	return registry
}

// Register adds a policy to the registry
func (r *PolicyRegistry) Register(policy Policy) {
	r.policies[policy.ID()] = policy
}

// Get retrieves a policy by ID
func (r *PolicyRegistry) Get(id string) (Policy, bool) {
	policy, ok := r.policies[id]
	return policy, ok
}

// GetAll returns all registered policies
func (r *PolicyRegistry) GetAll() []Policy {
	result := make([]Policy, 0, len(r.policies))
	for _, policy := range r.policies {
		result = append(result, policy)
	}
	return result
}

// GetEnabled returns only enabled policies
func (r *PolicyRegistry) GetEnabled() []Policy {
	var result []Policy
	for _, policy := range r.policies {
		if policy.IsEnabled() {
			result = append(result, policy)
		}
	}
	return result
}

// Enable enables a policy by ID
func (r *PolicyRegistry) Enable(id string) error {
	policy, ok := r.policies[id]
	if !ok {
		return fmt.Errorf("policy not found: %s", id)
	}
	policy.SetEnabled(true)
	return nil
}

// Disable disables a policy by ID
func (r *PolicyRegistry) Disable(id string) error {
	policy, ok := r.policies[id]
	if !ok {
		return fmt.Errorf("policy not found: %s", id)
	}
	policy.SetEnabled(false)
	return nil
}

// LoadConfiguration loads policy configuration from a config file
func (m *GuardrailsManager) LoadConfiguration() error {
	configPath := filepath.Join(m.config.StorageDir, "config.json")
	data, err := os.ReadFile(configPath)
	if err != nil {
		return err
	}
	return json.Unmarshal(data, m.config)
}

// SaveConfiguration saves policy configuration to a file
func (m *GuardrailsManager) SaveConfiguration() error {
	configPath := filepath.Join(m.config.StorageDir, "config.json")
	data, err := json.MarshalIndent(m.config, "", "  ")
	if err != nil {
		return err
	}
	return os.WriteFile(configPath, data, 0644)
}

// GuardrailsManager provides a high-level interface for the guardrails system
type GuardrailsManager struct {
	registry    *PolicyRegistry
	interceptor *Interceptor
	config      *GuardrailsConfig
}

// GuardrailsConfig holds configuration for the guardrails system
type GuardrailsConfig struct {
	Enabled       bool   `json:"enabled" yaml:"enabled"`
	StorageDir    string `json:"storage_dir" yaml:"storage_dir"`
	Interactive   bool   `json:"interactive" yaml:"interactive"`
	AutoRollback  bool   `json:"auto_rollback" yaml:"auto_rollback"`
	ConfirmDanger bool   `json:"confirm_danger" yaml:"confirm_danger"`
}

// DefaultConfig returns the default guardrails configuration
func DefaultConfig() *GuardrailsConfig {
	homeDir, _ := os.UserHomeDir()
	storageDir := filepath.Join(homeDir, ".attest", "checkpoints")

	config := &GuardrailsConfig{
		Enabled:       true,
		StorageDir:    storageDir,
		Interactive:   true,
		AutoRollback:  true,
		ConfirmDanger: true,
	}

	// Try to load existing config
	configPath := filepath.Join(storageDir, "config.json")
	if data, err := os.ReadFile(configPath); err == nil {
		if err := json.Unmarshal(data, config); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: Failed to parse guardrails config: %v\n", err)
		}
	}

	return config
}

// NewGuardrailsManager creates a new guardrails manager with default configuration
func NewGuardrailsManager() *GuardrailsManager {
	config := DefaultConfig()
	return NewGuardrailsManagerWithConfig(config)
}

// NewGuardrailsManagerWithConfig creates a new guardrails manager with custom configuration
func NewGuardrailsManagerWithConfig(config *GuardrailsConfig) *GuardrailsManager {
	// Ensure storage and policies directories exist
	if err := os.MkdirAll(config.StorageDir, 0755); err != nil {
		fmt.Printf("Warning: failed to create storage directory: %v\n", err)
	}
	policiesDir := filepath.Join(filepath.Dir(config.StorageDir), "policies")
	if err := os.MkdirAll(policiesDir, 0755); err != nil {
		fmt.Printf("Warning: failed to create policies directory: %v\n", err)
	}

	// Create checkpoint manager
	checkpointManager := NewCheckpointManager(config.StorageDir)

	// Create interceptor
	interceptor := NewInterceptor(checkpointManager, config.Interactive)

	// Register all policies
	registry := NewPolicyRegistry()
	for _, p := range registry.GetAll() {
		interceptor.AddPolicy(p)
	}

	manager := &GuardrailsManager{
		registry:    registry,
		interceptor: interceptor,
		config:      config,
	}

	// Load custom policies from disk
	if err := manager.loadCustomPolicies(); err != nil {
		fmt.Printf("Warning: failed to load custom policies: %v\n", err)
	}

	return manager
}

func (m *GuardrailsManager) loadCustomPolicies() error {
	policiesDir := filepath.Join(filepath.Dir(m.config.StorageDir), "policies")
	if _, err := os.Stat(policiesDir); os.IsNotExist(err) {
		return nil // No custom policies yet
	}

	// Use the policy package's loader
	rawPolicies, err := policy.LoadPoliciesFromDir(policiesDir)
	if err != nil {
		return err
	}

	for _, p := range rawPolicies {
		m.AddPolicy(policies.NewCustomPolicy(p))
	}

	return nil
}

// SetEnabled enables or disables guardrails
func (m *GuardrailsManager) SetEnabled(enabled bool) error {
	m.config.Enabled = enabled
	m.interceptor.SetEnabled(enabled)
	return m.SaveConfiguration()
}

// GetPolicies returns all registered policies
func (m *GuardrailsManager) GetPolicies() []Policy {
	return m.registry.GetAll()
}

// EnablePolicy enables a specific policy
func (m *GuardrailsManager) EnablePolicy(policyID string) error {
	return m.registry.Enable(policyID)
}

// DisablePolicy disables a specific policy
func (m *GuardrailsManager) DisablePolicy(policyID string) error {
	return m.registry.Disable(policyID)
}

// AddPolicy adds a new policy to the manager and interceptor
func (m *GuardrailsManager) AddPolicy(policy Policy) {
	m.registry.Register(policy)
	m.interceptor.AddPolicy(policy)
}

// SavePolicy saves a custom policy to disk
func (m *GuardrailsManager) SavePolicy(policyID string, rawYAML []byte) error {
	policiesDir := filepath.Join(filepath.Dir(m.config.StorageDir), "policies")
	filename := fmt.Sprintf("%s.yaml", policyID)
	return os.WriteFile(filepath.Join(policiesDir, filename), rawYAML, 0644)
}

// Execute runs a command with guardrail protection
func (m *GuardrailsManager) Execute(ctx context.Context, command string, args []string) (*ExecutionResult, error) {
	return m.interceptor.ExecuteWithGuardrails(ctx, command, args)
}

// CreateCheckpoint manually creates a checkpoint
func (m *GuardrailsManager) CreateCheckpoint(ctx context.Context, op *Operation) (*Checkpoint, error) {
	return m.interceptor.manager.CreateCheckpoint(ctx, op)
}

// ListCheckpoints returns all available checkpoints
func (m *GuardrailsManager) ListCheckpoints(ctx context.Context) ([]*Checkpoint, error) {
	return m.interceptor.manager.ListCheckpoints(ctx)
}

// Rollback restores state from a checkpoint
func (m *GuardrailsManager) Rollback(ctx context.Context, checkpointID string) (*RollbackResult, error) {
	return m.interceptor.manager.Rollback(ctx, checkpointID)
}

// GetCheckpoint retrieves a checkpoint by ID
func (m *GuardrailsManager) GetCheckpoint(ctx context.Context, checkpointID string) (*Checkpoint, error) {
	return m.interceptor.manager.GetCheckpoint(ctx, checkpointID)
}

// DeleteCheckpoint removes a checkpoint
func (m *GuardrailsManager) DeleteCheckpoint(ctx context.Context, checkpointID string) error {
	return m.interceptor.manager.DeleteCheckpoint(ctx, checkpointID)
}

// SetConfig updates the guardrails configuration
func (m *GuardrailsManager) SetConfig(config *GuardrailsConfig) {
	m.config = config
	m.interceptor.SetEnabled(config.Enabled)
	m.interceptor.interactive = config.Interactive
}

// GetConfig returns the current configuration
func (m *GuardrailsManager) GetConfig() *GuardrailsConfig {
	return m.config
}

// Global instance for application-wide use
var globalManager *GuardrailsManager

// InitGlobalManager initializes the global guardrails manager
func InitGlobalManager() {
	globalManager = NewGuardrailsManager()
}

// GetGlobalManager returns the global guardrails manager
func GetGlobalManager() *GuardrailsManager {
	if globalManager == nil {
		InitGlobalManager()
	}
	return globalManager
}

// SetGlobalManager sets the global guardrails manager
func SetGlobalManager(manager *GuardrailsManager) {
	globalManager = manager
}
