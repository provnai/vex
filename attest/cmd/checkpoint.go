package cmd

import (
	"context"
	"fmt"
	"os"
	"strings"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"

	"github.com/provnai/attest/pkg/guardrails"
)

var checkpointCmd = &cobra.Command{
	Use:   "checkpoint",
	Short: "Checkpoint management",
	Long:  `Create, list, and manage system checkpoints for rollback capability.`,
}

var checkpointCreateCmd = &cobra.Command{
	Use:   "create [description]",
	Short: "Create a manual checkpoint",
	Long: `Create a checkpoint of the current system state.
This can be used to manually save state before risky operations.

Example:
  attest checkpoint create "before migration"
  attest checkpoint create "pre-deploy snapshot"`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		description := "manual checkpoint"
		if len(args) > 0 {
			description = args[0]
		}

		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		op := &guardrails.Operation{
			ID:         fmt.Sprintf("manual_%d", time.Now().UnixNano()),
			Type:       "manual",
			Command:    "checkpoint",
			Args:       []string{},
			Env:        getEnvMap(),
			WorkingDir: getWorkingDir(),
			Metadata:   map[string]interface{}{},
		}

		checkpoint, err := manager.CreateCheckpoint(ctx, op)
		if err != nil {
			return fmt.Errorf("failed to create checkpoint: %w", err)
		}

		green := color.New(color.FgGreen).SprintFunc()
		cyan := color.New(color.FgCyan).SprintFunc()

		fmt.Println()
		fmt.Printf("%s Checkpoint created: %s\n", green("✓"), checkpoint.ID)
		fmt.Printf("  Description: %s\n", cyan(description))
		fmt.Printf("  Files:       %d\n", len(checkpoint.FileStates))
		fmt.Printf("  DB States:   %d\n", len(checkpoint.DBStates))
		fmt.Printf("  Size:        %d bytes\n", checkpoint.Size)

		return nil
	},
}

var checkpointListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all checkpoints",
	Long:  "Display all available checkpoints with their status and metadata.",
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoints, err := manager.ListCheckpoints(ctx)
		if err != nil {
			return fmt.Errorf("failed to list checkpoints: %w", err)
		}

		cyan := color.New(color.FgCyan).SprintFunc()
		green := color.New(color.FgGreen).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()
		red := color.New(color.FgRed).SprintFunc()

		fmt.Println()
		fmt.Println(cyan("System Checkpoints"))
		fmt.Println("===================")
		fmt.Println()

		if len(checkpoints) == 0 {
			fmt.Println("No checkpoints found.")
			fmt.Println("Run 'attest checkpoint create' to create your first checkpoint.")
			return nil
		}

		fmt.Printf("%-15s %-25s %-10s %s\n", cyan("ID"), cyan("Created"), cyan("Status"), cyan("Files"))
		fmt.Println(strings.Repeat("-", 70))

		for _, cp := range checkpoints {
			status := green("ACTIVE")
			if cp.Type == "rolled_back" {
				status = yellow("ROLLED_BACK")
			} else if cp.Type == "expired" {
				status = red("EXPIRED")
			}

			created := cp.CreatedAt.Format("2006-01-02 15:04")
			fmt.Printf("%-15s %-25s %-10s %d\n", cp.ID, created, status, len(cp.FileStates))
		}

		return nil
	},
}

var checkpointShowCmd = &cobra.Command{
	Use:   "show <checkpoint_id>",
	Short: "Show checkpoint details",
	Long:  "Display detailed information about a specific checkpoint.",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		checkpointID := args[0]

		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoint, err := manager.GetCheckpoint(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("checkpoint not found: %s", checkpointID)
		}

		cyan := color.New(color.FgCyan).SprintFunc()
		green := color.New(color.FgGreen).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()

		fmt.Println()
		fmt.Printf("Checkpoint: %s\n", checkpoint.ID)
		fmt.Println("===========")
		fmt.Println()
		fmt.Printf("ID:          %s\n", checkpoint.ID)
		fmt.Printf("Operation:   %s %s\n", checkpoint.Data["command"], checkpoint.Data["args"])
		fmt.Printf("Created:     %s\n", checkpoint.CreatedAt.Format(time.RFC3339))
		fmt.Printf("Type:        %s\n", checkpoint.Type)
		fmt.Printf("Files:       %d\n", len(checkpoint.FileStates))
		fmt.Printf("DB States:   %d\n", len(checkpoint.DBStates))
		fmt.Printf("Size:        %d bytes\n", checkpoint.Size)
		fmt.Println()

		if len(checkpoint.FileStates) > 0 {
			fmt.Println(cyan("Tracked Files:"))
			for i, fs := range checkpoint.FileStates {
				if i >= 10 {
					fmt.Printf("  ... and %d more files\n", len(checkpoint.FileStates)-10)
					break
				}
				exists := green("exists")
				if !fs.Exists {
					exists = yellow("deleted")
				}
				fmt.Printf("  %s - %s (%s)\n", fs.Path, exists, fs.Hash[:8])
			}
		}

		return nil
	},
}

var checkpointDeleteCmd = &cobra.Command{
	Use:   "delete <checkpoint_id>",
	Short: "Delete a checkpoint",
	Long:  "Remove a checkpoint from the system.",
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		checkpointID := args[0]

		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		err := manager.DeleteCheckpoint(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("failed to delete checkpoint: %w", err)
		}

		green := color.New(color.FgGreen).SprintFunc()
		fmt.Printf("%s Checkpoint deleted: %s\n", green("✓"), checkpointID)

		return nil
	},
}

func init() {
	checkpointCmd.AddCommand(checkpointCreateCmd)
	checkpointCmd.AddCommand(checkpointListCmd)
	checkpointCmd.AddCommand(checkpointShowCmd)
	checkpointCmd.AddCommand(checkpointDeleteCmd)

	rootCmd.AddCommand(checkpointCmd)
}

func getEnvMap() map[string]string {
	env := make(map[string]string)
	for _, e := range os.Environ() {
		if i := len(e) - 1; i > 0 {
			for j := 0; j < i; j++ {
				if e[j] == '=' {
					env[e[:j]] = e[j+1:]
					break
				}
			}
		}
	}
	return env
}

func getWorkingDir() string {
	dir, _ := os.Getwd()
	return dir
}
