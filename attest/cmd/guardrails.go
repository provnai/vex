package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/fatih/color"
	"github.com/spf13/cobra"

	"github.com/provnai/attest/pkg/guardrails"
	"github.com/provnai/attest/pkg/guardrails/policies"
)

var guardrailsCmd = &cobra.Command{
	Use:   "guardrails",
	Short: "Guardrails safety system management",
	Long: `Attest Guardrails - The Safety Net for AI Agents

Provides policy enforcement, checkpoint creation, and automatic rollback
capabilities to prevent disasters during command execution.`,
	PersistentPreRunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		storageDir := filepath.Join(cfg.DataDir, "checkpoints")
		if manager.GetConfig().StorageDir != storageDir {
			config := manager.GetConfig()
			config.StorageDir = storageDir
			manager.SetConfig(config)
		}
		return nil
	},
}

var guardrailsEnableCmd = &cobra.Command{
	Use:   "enable",
	Short: "Enable guardrails protection",
	Long:  "Enable the guardrails safety system for all command executions.",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		if err := manager.SetEnabled(true); err != nil {
			return fmt.Errorf("failed to enable guardrails: %w", err)
		}

		green := color.New(color.FgGreen).SprintFunc()
		fmt.Printf("%s Guardrails enabled\n", green("✓"))

		return nil
	},
}

var guardrailsDisableCmd = &cobra.Command{
	Use:   "disable",
	Short: "Disable guardrails protection",
	Long:  "Disable the guardrails safety system (use with caution!).",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		if err := manager.SetEnabled(false); err != nil {
			return fmt.Errorf("failed to disable guardrails: %w", err)
		}

		yellow := color.New(color.FgYellow).SprintFunc()
		fmt.Printf("%s Guardrails disabled - proceed with caution!\n", yellow("⚠"))

		return nil
	},
}

var guardrailsPoliciesCmd = &cobra.Command{
	Use:   "policies",
	Short: "List active policies",
	Long:  "Display all registered policies and their current status.",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		policies := manager.GetPolicies()

		cyan := color.New(color.FgCyan).SprintFunc()
		green := color.New(color.FgGreen).SprintFunc()
		red := color.New(color.FgRed).SprintFunc()

		fmt.Println()
		fmt.Println(cyan("Active Guardrail Policies"))
		fmt.Println("===========================")
		fmt.Println()

		for _, policy := range policies {
			status := green("ENABLED")
			if !policy.IsEnabled() {
				status = red("DISABLED")
			}

			fmt.Printf("Policy: %s\n", policy.Name())
			fmt.Printf("  ID:          %s\n", policy.ID())
			fmt.Printf("  Status:      %s\n", status)
			fmt.Printf("  Description: %s\n", policy.Description())
			fmt.Println()
		}

		return nil
	},
}

var guardrailsAddCmd = &cobra.Command{
	Use:   "add <policy.yaml>",
	Short: "Add a custom policy",
	Long: `Add a custom policy from a YAML configuration file.

Example policy.yaml:
  id: my-custom-policy
  name: Custom Policy
  description: Blocks specific operations
  enabled: true
  patterns:
    - "rm -rf /tmp/*"
  action: block`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		configPath := args[0]

		if _, err := os.Stat(configPath); os.IsNotExist(err) {
			return fmt.Errorf("policy file not found: %s", configPath)
		}

		// Task 7: Load and register the custom policy
		manager := guardrails.GetGlobalManager()

		yamlBytes, err := os.ReadFile(configPath)
		if err != nil {
			return fmt.Errorf("failed to read policy file: %w", err)
		}

		customPolicy, err := policies.LoadCustomPolicy(configPath)
		if err != nil {
			return fmt.Errorf("failed to load custom policy: %w", err)
		}

		manager.AddPolicy(customPolicy)

		// Persist for future sessions
		if err := manager.SavePolicy(customPolicy.ID(), yamlBytes); err != nil {
			fmt.Printf("Warning: failed to persist policy to disk: %v\n", err)
		}

		green := color.New(color.FgGreen).SprintFunc()
		fmt.Printf("%s Custom policy '%s' (%s) loaded and enabled\n", green("✓"), customPolicy.Name(), customPolicy.ID())

		return nil
	},
}

var guardrailsStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show guardrails status",
	Long:  "Display the current guardrails configuration and status.",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		config := manager.GetConfig()

		cyan := color.New(color.FgCyan).SprintFunc()
		green := color.New(color.FgGreen).SprintFunc()
		red := color.New(color.FgRed).SprintFunc()

		fmt.Println()
		fmt.Println(cyan("Guardrails Status"))
		fmt.Println("==================")
		fmt.Println()

		enabled := green("ENABLED")
		if !config.Enabled {
			enabled = red("DISABLED")
		}
		fmt.Printf("Status:        %s\n", enabled)
		fmt.Printf("Interactive:   %v\n", config.Interactive)
		fmt.Printf("Auto Rollback: %v\n", config.AutoRollback)
		fmt.Printf("Confirm Danger: %v\n", config.ConfirmDanger)
		fmt.Printf("Storage Dir:   %s\n", config.StorageDir)

		return nil
	},
}

func init() {
	guardrailsCmd.AddCommand(guardrailsEnableCmd)
	guardrailsCmd.AddCommand(guardrailsDisableCmd)
	guardrailsCmd.AddCommand(guardrailsPoliciesCmd)
	guardrailsCmd.AddCommand(guardrailsAddCmd)
	guardrailsCmd.AddCommand(guardrailsStatusCmd)

	rootCmd.AddCommand(guardrailsCmd)
}
