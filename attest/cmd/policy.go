package cmd

import (
	"encoding/json"
	"fmt"
	"os"

	"github.com/provnai/attest/pkg/policy"
	"github.com/spf13/cobra"
)

var (
	policyJSON        bool
	policyCheckType   string
	policyCheckTarget string
	policyCheckAgent  string
)

func init() {
	policyCmd.AddCommand(policyCheckCmd)
	policyCmd.AddCommand(policyAddCmd)
	policyCmd.AddCommand(policyListCmd)
	policyCmd.AddCommand(policyRemoveCmd)

	policyCmd.PersistentFlags().BoolVar(&policyJSON, "json", false, "output as JSON")

	policyCheckCmd.Flags().StringVar(&policyCheckType, "type", "command", "action type to check")
	policyCheckCmd.Flags().StringVar(&policyCheckTarget, "target", "", "action target to check")
	policyCheckCmd.Flags().StringVar(&policyCheckAgent, "agent", "", "agent ID for context")
}

var policyCmd = &cobra.Command{
	Use:   "policy",
	Short: "Manage policies",
	Long:  `Configure and manage safety policies for agent actions.`,
}

var policyCheckCmd = &cobra.Command{
	Use:   "check",
	Short: "Check action against policies",
	Long:  `Test if an action would be allowed or blocked by current policies.`,
	Run: func(cmd *cobra.Command, args []string) {
		target := policyCheckTarget
		if len(args) > 0 && target == "" {
			target = args[0]
		}

		if target == "" {
			fmt.Println("Error: target (command or resource) is required")
			os.Exit(1)
		}

		engine := policy.NewPolicyEngine()
		ctx := policy.ActionContext{
			Type:    policyCheckType,
			Target:  target,
			AgentID: policyCheckAgent,
		}

		allowed, results := engine.ShouldAllow(ctx)

		if policyJSON {
			output := map[string]interface{}{
				"allowed": allowed,
				"results": results,
			}
			data, _ := json.MarshalIndent(output, "", "  ")
			fmt.Println(string(data))
			return
		}

		fmt.Printf("Policy Check Results:\n")
		fmt.Printf("Action:  %s %s\n", policyCheckType, target)
		fmt.Printf("Status:  ")
		if allowed {
			fmt.Println("ALLOWED ✓")
		} else {
			fmt.Println("BLOCKED ✗")
		}

		fmt.Println("\nPolicies Evaluated:")
		for _, r := range results {
			status := "PASS"
			if r.Matched {
				if r.Action == policy.PolicyActionBlock {
					status = "BLOCK"
				} else {
					status = "WARN"
				}
			}
			fmt.Printf("- [%s] %s: %s\n", status, r.PolicyID, r.Message)
		}
	},
}

var policyAddCmd = &cobra.Command{
	Use:   "add [file]",
	Short: "Add a policy",
	Long:  `Add a policy from a YAML file.`,
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		file := args[0]
		p, err := policy.LoadPolicyFromFile(file)
		if err != nil {
			fmt.Printf("Error loading policy: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("Policy loaded: %s (%s)\n", p.Name, p.ID)
		fmt.Printf("(Full add implementation coming)")
	},
}

var policyListCmd = &cobra.Command{
	Use:   "list",
	Short: "List policies",
	Long:  `List all active policies.`,
	Run: func(cmd *cobra.Command, args []string) {
		engine := policy.NewPolicyEngine()
		policies := engine.ListPolicies()

		if policyJSON {
			data, _ := json.MarshalIndent(policies, "", "  ")
			fmt.Println(string(data))
			return
		}

		fmt.Printf("%-25s %-30s %-10s %-8s\n", "ID", "NAME", "ACTION", "ENABLED")
		fmt.Printf("%-25s %-30s %-10s %-8s\n", "-------------------------", "------------------------------", "----------", "--------")
		for _, p := range policies {
			enabled := "Yes"
			if !p.Enabled {
				enabled = "No"
			}
			name := p.Name
			if len(name) > 28 {
				name = name[:28] + ".."
			}
			id := p.ID
			if len(id) > 24 {
				id = id[:24]
			}
			fmt.Printf("%-25s %-30s %-10s %-8s\n", id, name, p.Action, enabled)
		}
		fmt.Printf("\nTotal: %d policies\n", len(policies))
	},
}

var policyRemoveCmd = &cobra.Command{
	Use:   "remove [id]",
	Short: "Remove a policy",
	Long:  `Remove a policy by ID.`,
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		fmt.Printf("Would remove policy: %s\n", args[0])
		fmt.Printf("(Full remove implementation coming)")
	},
}
