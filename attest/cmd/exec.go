package cmd

import (
	"context"
	"fmt"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/provnai/attest/pkg/exec"
	"github.com/provnai/attest/pkg/guardrails"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
)

var (
	execReversible bool
	execIntent     string
	execAgent      string
	execDryRun     bool
	execBackupType string
	execEnv        string
)

func init() {
	execRunCmd.Flags().BoolVar(&execReversible, "reversible", false, "make this execution reversible")
	execRunCmd.Flags().StringVar(&execIntent, "intent", "", "link to an intent ID")
	execRunCmd.Flags().StringVar(&execAgent, "agent", "", "agent ID (required for signing)")
	execRunCmd.Flags().BoolVar(&execDryRun, "dry-run", false, "show what would happen without executing")
	execRunCmd.Flags().StringVar(&execBackupType, "backup", "file", "backup type (file, dir, none)")
	execRunCmd.Flags().StringVar(&execEnv, "env", "development", "environment (development, staging, production)")

	execCmd.AddCommand(execRunCmd)
	execCmd.AddCommand(execRollbackCmd)
	execCmd.AddCommand(execHistoryCmd)
}

var execCmd = &cobra.Command{
	Use:   "exec",
	Short: "Execute reversible commands",
	Long:  `Execute commands with automatic backup and optional reversibility.`,
}

var execRunCmd = &cobra.Command{
	Use:   "run [command...]",
	Short: "Run a reversible command",
	Long: `Execute a command with optional reversibility. Creates automatic backups
for file modifications when --reversible is specified.`,
	Example: `
  # Simple command
  attest exec run "echo hello"

  # Reversible command with backup
  attest exec run --reversible "python migrate.py"

  # Dry run to see what would happen
  attest exec run --dry-run "rm important.txt"

  # With agent identity and intent
  attest exec run --agent aid:1234 --intent int:abcd --reversible "python script.py"`,
	Args: cobra.MinimumNArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runExecRun(args); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var execRollbackCmd = &cobra.Command{
	Use:   "rollback [id]",
	Short: "Rollback an action",
	Long:  `Reverse a previously executed reversible action.`,
	Example: `
  # Rollback last action
  attest exec rollback last

  # Rollback specific action
  attest exec rollback exec:12345678`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runExecRollback(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var execHistoryCmd = &cobra.Command{
	Use:   "history",
	Short: "Show execution history",
	Long:  `Show all reversible actions with their status.`,
	Example: `
  # Show history
  attest exec history

  # Show pending rollbacks
  attest exec history --pending`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runExecHistory(); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

func runExecRun(args []string) error {
	manager := guardrails.GetGlobalManager()

	// Prepare the command and its arguments
	var command string
	var cmdArgs []string
	if len(args) > 0 {
		command = args[0]
		if len(args) > 1 {
			cmdArgs = args[1:]
		}
	}

	fmt.Printf("Executing with Guardrails: %s %s\n", command, strings.Join(cmdArgs, " "))

	ctx := context.Background()
	result, err := manager.Execute(ctx, command, cmdArgs)
	if err != nil {
		return err
	}

	if result.Blocked {
		fmt.Printf("✗ Blocked by Guardrails: %s\n", result.BlockedBy)
		return nil
	}

	if result.Success {
		fmt.Printf("✓ Executed successfully\n")
		if result.Checkpoint != nil {
			fmt.Printf("  Checkpoint created: %s\n", result.Checkpoint.ID)
		}
	} else {
		fmt.Printf("✗ Execution failed (Exit Code: %d)\n", result.ExitCode)
		if result.Stderr != "" {
			fmt.Printf("  Error: %s\n", result.Stderr)
		}
	}

	return nil
}

func runExecRollback(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return err
	}
	defer db.Close()

	if id == "last" {
		var lastID string
		err = db.QueryRow(
			`SELECT id FROM reversible_actions WHERE status = ? ORDER BY created_at DESC LIMIT 1`,
			exec.StatusExecuted,
		).Scan(&lastID)
		if err != nil {
			return fmt.Errorf("no pending rollbacks found")
		}
		id = lastID
	}

	var command, backupPath, originalPath string
	var status exec.ReversibleStatus
	err = db.QueryRow(
		`SELECT command, backup_path, working_dir, status FROM reversible_actions WHERE id = ?`,
		id,
	).Scan(&command, &backupPath, &originalPath, &status)

	if err != nil {
		return fmt.Errorf("action not found: %s", id)
	}

	if status == exec.StatusRolledBack {
		return fmt.Errorf("action already rolled back: %s", id)
	}

	executor, err := exec.NewExecutor(cfg.BackupDir)
	if err != nil {
		return err
	}

	if backupPath != "" && originalPath != "" {
		if err := executor.Rollback(id, backupPath, originalPath); err != nil {
			return fmt.Errorf("rollback failed: %w", err)
		}
	}

	if _, err := db.Exec(
		`UPDATE reversible_actions SET status = ?, rolled_back_at = ? WHERE id = ?`,
		exec.StatusRolledBack, time.Now().UTC().Format(time.RFC3339), id,
	); err != nil {
		fmt.Printf("Warning: failed to update action status in database: %v\n", err)
	}

	fmt.Printf("✓ Rolled back: %s\n", command)
	fmt.Printf("  Action ID: %s\n", id)
	return nil
}

func runExecHistory() error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return err
	}
	defer db.Close()

	limit := 50

	query := `SELECT id, backup_path, status, created_at, rolled_back_at FROM reversible_actions ORDER BY created_at DESC LIMIT ?`
	rows, err := db.Query(query, limit)
	if err != nil {
		return fmt.Errorf("failed to query history: %w", err)
	}
	defer rows.Close()

	type actionRow struct {
		ID           string
		BackupPath   string
		Status       exec.ReversibleStatus
		CreatedAt    string
		RolledBackAt *string
	}

	var actions []actionRow
	for rows.Next() {
		var r actionRow
		var rolledBack *string
		if err := rows.Scan(&r.ID, &r.BackupPath, &r.Status, &r.CreatedAt, &rolledBack); err != nil {
			return fmt.Errorf("failed to scan row: %w", err)
		}
		r.RolledBackAt = rolledBack
		actions = append(actions, r)
	}

	fmt.Printf("%-20s %-8s %-15s %s\n", "ID", "STATUS", "BACKUP", "CREATED")
	fmt.Printf("%-20s %-8s %-15s %s\n", "----", "------", "------", "-------")
	for _, a := range actions {
		statusIcon := "○"
		if a.Status == exec.StatusExecuted {
			statusIcon = "✓"
		} else if a.Status == exec.StatusRolledBack {
			statusIcon = "↩"
		} else if a.Status == exec.StatusFailed {
			statusIcon = "✗"
		}

		backup := "none"
		if a.BackupPath != "" {
			backup = filepath.Base(a.BackupPath)
		}

		rollback := ""
		if a.RolledBackAt != nil {
			rollback = " [rolled back]"
		}

		fmt.Printf("%-20s %s %-15s %s%s\n", a.ID[:20], statusIcon, backup, a.CreatedAt[:10], rollback)
	}

	return nil
}

// containsDangerousPatterns and confirmDangerous are now handled by the guardrails package
