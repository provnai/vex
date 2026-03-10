package cmd

import (
	"context"
	"fmt"

	"github.com/fatih/color"
	"github.com/spf13/cobra"

	"github.com/provnai/attest/pkg/guardrails"
)

var rollbackCmd = &cobra.Command{
	Use:   "rollback",
	Short: "Rollback system state",
	Long:  `Restore system state from a previously created checkpoint.`,
}

var lastSafe bool

var rollbackToCheckpointCmd = &cobra.Command{
	Use:   "to <checkpoint_id>",
	Short: "Rollback to a specific checkpoint",
	Long: `Restore system state from a specific checkpoint.

Example:
  attest rollback to chk:abc123
  attest rollback to chk:def456 --force`,
	Args: cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		checkpointID := args[0]

		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoint, err := manager.GetCheckpoint(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("checkpoint not found: %s", checkpointID)
		}

		green := color.New(color.FgGreen).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()
		red := color.New(color.FgRed).SprintFunc()

		fmt.Println()
		fmt.Printf("Preparing to rollback to checkpoint: %s\n", checkpointID)
		fmt.Println("======================================")
		fmt.Println()
		fmt.Printf("Checkpoint:    %s\n", checkpoint.ID)
		fmt.Printf("Created:       %s\n", checkpoint.CreatedAt.Format("2006-01-02 15:04:05"))
		fmt.Printf("Type:          %s\n", checkpoint.Type)
		fmt.Printf("Files to restore: %d\n", len(checkpoint.FileStates))
		fmt.Printf("DB states to restore: %d\n", len(checkpoint.DBStates))
		fmt.Println()

		fmt.Println(yellow("⚠ This will overwrite current system state with checkpoint data."))
		fmt.Println(red("⚠ Any changes made after this checkpoint will be lost."))
		fmt.Println()

		if !lastSafe {
			confirm, err := confirmRollback()
			if err != nil {
				return err
			}
			if !confirm {
				fmt.Println("Rollback cancelled.")
				return nil
			}
		}

		fmt.Printf("%s Rolling back to checkpoint %s\n", yellow("⟲"), checkpointID)

		result, err := manager.Rollback(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("rollback failed: %w", err)
		}

		fmt.Println()
		if result.Success {
			fmt.Printf("%s Rollback complete\n", green("✓"))
			fmt.Printf("  Files restored:    %d\n", result.RestoredFiles)
			fmt.Printf("  DB states restored: %d\n", result.RestoredDB)
			fmt.Printf("  Duration:          %v\n", result.Duration)

			if len(result.Errors) > 0 {
				yellow := color.New(color.FgYellow).SprintFunc()
				fmt.Printf("\n%s Warnings:\n", yellow("⚠"))
				for _, e := range result.Errors {
					fmt.Printf("  - %v\n", e)
				}
			}
		} else {
			red := color.New(color.FgRed).SprintFunc()
			fmt.Printf("%s Rollback completed with errors\n", red("✗"))
			for _, e := range result.Errors {
				fmt.Printf("  - %v\n", e)
			}
		}

		return nil
	},
}

var rollbackLastSafeCmd = &cobra.Command{
	Use:   "last-safe",
	Short: "Rollback to the last safe checkpoint",
	Long: `Automatically find and rollback to the most recent ACTIVE checkpoint.

This is useful when something goes wrong and you want to quickly
restore the last known good state.`,
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoints, err := manager.ListCheckpoints(ctx)
		if err != nil {
			return fmt.Errorf("failed to list checkpoints: %w", err)
		}

		var lastActive *guardrails.Checkpoint
		for _, cp := range checkpoints {
			if cp.Type != "rolled_back" && cp.Type != "expired" {
				if lastActive == nil || cp.CreatedAt.After(lastActive.CreatedAt) {
					lastActive = cp
				}
			}
		}

		if lastActive == nil {
			return fmt.Errorf("no active checkpoints found")
		}

		green := color.New(color.FgGreen).SprintFunc()

		fmt.Println()
		fmt.Printf("Last safe checkpoint found: %s\n", lastActive.ID)
		fmt.Printf("Created: %s\n", lastActive.CreatedAt.Format("2006-01-02 15:04:05"))
		fmt.Println()

		result, err := manager.Rollback(ctx, lastActive.ID)
		if err != nil {
			return fmt.Errorf("rollback failed: %w", err)
		}

		fmt.Printf("%s Rolled back to last safe checkpoint\n", green("✓"))
		fmt.Printf("  Files restored:    %d\n", result.RestoredFiles)
		return nil
	},
}

var rollbackStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Check rollback status",
	Long:  "Check the status of recent rollbacks and checkpoints.",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoints, err := manager.ListCheckpoints(ctx)
		if err != nil {
			return fmt.Errorf("failed to list checkpoints: %w", err)
		}

		cyan := color.New(color.FgCyan).SprintFunc()

		fmt.Println()
		fmt.Println(cyan("Rollback Status"))
		fmt.Println("================")
		fmt.Println()

		activeCount := 0
		rolledBackCount := 0

		for _, cp := range checkpoints {
			if cp.Type == "rolled_back" {
				rolledBackCount++
			} else {
				activeCount++
			}
		}

		fmt.Printf("Active checkpoints:    %d\n", activeCount)
		fmt.Printf("Rolled back checkpoints: %d\n", rolledBackCount)
		fmt.Println()

		if rolledBackCount > 0 {
			fmt.Println(cyan("Recently Rolled Back:"))
			for _, cp := range checkpoints {
				if cp.Type == "rolled_back" {
					fmt.Printf("  - %s (%s)\n", cp.ID, cp.CreatedAt.Format("2006-01-02 15:04"))
				}
			}
		}

		return nil
	},
}

func init() {
	rollbackCmd.AddCommand(rollbackToCheckpointCmd)
	rollbackCmd.AddCommand(rollbackLastSafeCmd)
	rollbackCmd.AddCommand(rollbackStatusCmd)

	rollbackToCheckpointCmd.Flags().BoolVarP(&lastSafe, "force", "f", false, "Skip confirmation prompt")

	rootCmd.AddCommand(rollbackCmd)
}
