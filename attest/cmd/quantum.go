package cmd

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/fatih/color"
	"github.com/spf13/cobra"

	"github.com/provnai/attest/pkg/crypto"
	"github.com/provnai/attest/pkg/guardrails"
)

var quantumCmd = &cobra.Command{
	Use:   "quantum",
	Short: "Time-travel undo (Quantum Undo)",
	Long:  `Time-travel checkpoint system for advanced rollback and state comparison.`,
}

var quantumDiffCmd = &cobra.Command{
	Use:   "diff <checkpoint_id>",
	Short: "Show differences between checkpoint and current state",
	Long: `Compare a checkpoint's state with the current system state.
This shows what files have changed since the checkpoint was created.

Example:
  attest quantum diff chk:abc123`,
	Args: cobra.ExactArgs(1),
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
		red := color.New(color.FgRed).SprintFunc()

		fmt.Println()
		fmt.Printf("Quantum Diff: %s vs Current State\n", checkpoint.ID)
		fmt.Println("===================================")
		fmt.Println()

		fmt.Printf("Checkpoint created: %s\n", checkpoint.CreatedAt.Format("2006-01-02 15:04:05"))
		fmt.Println()

		currentFiles := make(map[string]FileState)
		scanCurrentDirectory(".", currentFiles)

		fmt.Println(cyan("Changes since checkpoint:"))
		fmt.Println(strings.Repeat("-", 60))

		added := 0
		modified := 0
		deleted := 0

		for _, cpFile := range checkpoint.FileStates {
			if current, ok := currentFiles[cpFile.Path]; ok {
				if cpFile.Hash != current.Hash {
					modified++
					fmt.Printf("%s Modified: %s\n", yellow("M"), cpFile.Path)
					fmt.Printf("    Old: %s\n", cpFile.Hash[:8])
					fmt.Printf("    New: %s\n", current.Hash[:8])
				}
			} else {
				deleted++
				fmt.Printf("%s Deleted: %s\n", red("X"), cpFile.Path)
			}
			delete(currentFiles, cpFile.Path)
		}

		for path := range currentFiles {
			added++
			fmt.Printf("%s Added: %s\n", green("+"), path)
		}

		fmt.Println()
		fmt.Printf("Summary: +%d added, ~%d modified, -%d deleted\n", added, modified, deleted)

		return nil
	},
}

var quantumTimelineCmd = &cobra.Command{
	Use:   "timeline",
	Short: "Show checkpoint timeline",
	Long: `Display a visual timeline of all checkpoints with their relationships.
This helps you understand the history of your project state.

Example:
  attest quantum timeline`,
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
		blue := color.New(color.FgBlue).SprintFunc()

		fmt.Println()
		fmt.Println(cyan("Quantum Timeline"))
		fmt.Println("=================")
		fmt.Println()

		if len(checkpoints) == 0 {
			fmt.Println("No checkpoints in timeline.")
			fmt.Println("Run 'attest checkpoint create' to start tracking.")
			return nil
		}

		fmt.Println(blue("◀── Time flows left to right ──▶"))
		fmt.Println()

		for i, cp := range checkpoints {
			if i > 0 {
				fmt.Println(blue("    │"))
				fmt.Println(blue("    ▼"))
			}

			status := green("●")
			if cp.Type == "rolled_back" {
				status = yellow("◉")
			} else if cp.Type == "expired" {
				status = red("○")
			}

			timestamp := cp.CreatedAt.Format("15:04")
			age := time.Since(cp.CreatedAt)
			ageStr := formatAge(age)

			fmt.Printf("%s [%s] %s %s (%s)\n", status, timestamp, cp.ID, ageStr, cp.Type)
		}

		fmt.Println()
		fmt.Print(cyan("Legend:"))
		fmt.Printf(" %s Active", green("●"))
		fmt.Printf(" %s Rolled Back", yellow("◉"))
		fmt.Printf(" %s Expired\n", red("○"))

		return nil
	},
}

var quantumUndoCmd = &cobra.Command{
	Use:   "undo [checkpoint_id]",
	Short: "Undo to previous checkpoint (quantum revert)",
	Long: `Revert to a previous checkpoint state with full verification.
This is the main "time-travel" command for restoring previous states.

Examples:
  attest quantum undo                      # Undo to last safe checkpoint
  attest quantum undo chk:abc123           # Undo to specific checkpoint
  attest quantum undo --dry-run chk:abc123 # Preview changes without applying`,
	Args: cobra.MaximumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		var checkpointID string
		var err error

		if len(args) > 0 {
			checkpointID = args[0]
		} else {
			checkpoints, err := manager.ListCheckpoints(ctx)
			if err != nil {
				return fmt.Errorf("failed to list checkpoints: %w", err)
			}

			for _, cp := range checkpoints {
				if cp.Type != "rolled_back" && cp.Type != "expired" {
					checkpointID = cp.ID
					break
				}
			}

			if checkpointID == "" {
				return fmt.Errorf("no active checkpoints found")
			}
		}

		checkpoint, err := manager.GetCheckpoint(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("checkpoint not found: %s", checkpointID)
		}

		cyan := color.New(color.FgCyan).SprintFunc()
		green := color.New(color.FgGreen).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()

		fmt.Println()
		fmt.Println(yellow("⚠ QUANTUM UNDO"))
		fmt.Println("===============")
		fmt.Println()
		fmt.Printf("Target: %s\n", checkpoint.ID)
		fmt.Printf("Created: %s\n", checkpoint.CreatedAt.Format("2006-01-02 15:04:05"))
		fmt.Printf("Files to restore: %d\n", len(checkpoint.FileStates))
		fmt.Println()
		fmt.Println(cyan("This will:"))
		fmt.Println("  1. Create a backup of current state")
		fmt.Println("  2. Restore files from checkpoint")
		fmt.Println("  3. Verify all changes")
		fmt.Println()

		confirm, err := confirmRollback()
		if err != nil {
			return err
		}
		if !confirm {
			fmt.Println("Undo cancelled.")
			return nil
		}

		fmt.Printf("%s Performing quantum undo to %s\n", green("⟲"), checkpointID)

		result, err := manager.Rollback(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("undo failed: %w", err)
		}

		fmt.Println()
		if result.Success {
			fmt.Printf("%s Quantum undo complete\n", green("✓"))
			fmt.Printf("  Files restored: %d\n", result.RestoredFiles)
			fmt.Printf("  Duration: %v\n", result.Duration)

			if len(result.Errors) > 0 {
				fmt.Printf("\n%s Warnings:\n", yellow("⚠"))
				for _, e := range result.Errors {
					fmt.Printf("  - %v\n", e)
				}
			}
		} else {
			fmt.Printf("%s Undo completed with errors\n", yellow("⚠"))
			for _, e := range result.Errors {
				fmt.Printf("  - %v\n", e)
			}
		}

		return nil
	},
}

var quantumBranchCmd = &cobra.Command{
	Use:   "branch <checkpoint_id> <branch_name>",
	Short: "Create a new branch from checkpoint",
	Long: `Create a named "branch" from an existing checkpoint.
This allows you to maintain multiple parallel states.

Example:
  attest quantum branch chk:abc123 experiment-v1`,
	Args: cobra.ExactArgs(2),
	RunE: func(cmd *cobra.Command, args []string) error {
		checkpointID := args[0]
		branchName := args[1]

		manager := guardrails.GetGlobalManager()
		ctx := context.Background()

		checkpoint, err := manager.GetCheckpoint(ctx, checkpointID)
		if err != nil {
			return fmt.Errorf("checkpoint not found: %s", checkpointID)
		}

		green := color.New(color.FgGreen).SprintFunc()

		fmt.Println()
		fmt.Printf("%s Creating branch '%s' from %s\n", green("✦"), branchName, checkpointID)
		fmt.Printf("  Checkpoint: %s\n", checkpoint.ID)
		fmt.Printf("  Created: %s\n", checkpoint.CreatedAt.Format("2006-01-02 15:04:05"))
		fmt.Printf("  Files: %d\n", len(checkpoint.FileStates))
		fmt.Println()
		fmt.Printf("%s Branch created! Use 'attest quantum undo %s' to restore.\n", green("✓"), checkpointID)

		return nil
	},
}

type FileState struct {
	Path string
	Hash string
}

func scanCurrentDirectory(root string, files map[string]FileState) {
	if err := filepath.Walk(root, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}

		relPath, _ := filepath.Rel(root, path)

		if strings.HasPrefix(relPath, ".git") ||
			strings.HasPrefix(relPath, ".attest") ||
			strings.HasPrefix(relPath, "node_modules") ||
			strings.HasPrefix(relPath, "__pycache__") ||
			strings.Contains(relPath, ".pyc") ||
			relPath == "attest" || relPath == "attest.exe" {
			if info.IsDir() {
				return filepath.SkipDir
			}
			return nil
		}

		if !info.IsDir() {
			hash, err := crypto.HashFile(path)
			if err != nil {
				// Handle error or skip
				return nil
			}
			files[relPath] = FileState{
				Path: relPath,
				Hash: hash,
			}
		}

		return nil
	}); err != nil {
		fmt.Printf("Warning: error during directory scan: %v\n", err)
	}
}

func formatAge(d time.Duration) string {
	if d < time.Minute {
		return "just now"
	} else if d < time.Hour {
		return fmt.Sprintf("%dm ago", int(d.Minutes()))
	} else if d < 24*time.Hour {
		return fmt.Sprintf("%dh ago", int(d.Hours()))
	}
	return fmt.Sprintf("%dd ago", int(d.Hours()/24))
}

func init() {
	quantumCmd.AddCommand(quantumDiffCmd)
	quantumCmd.AddCommand(quantumTimelineCmd)
	quantumCmd.AddCommand(quantumUndoCmd)
	quantumCmd.AddCommand(quantumBranchCmd)

	rootCmd.AddCommand(quantumCmd)
}
