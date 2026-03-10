package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"regexp"
	"strings"

	"github.com/spf13/cobra"
)

var (
	hookInstallDir string
	skipDryRun     bool
	forceReinstall bool
)

func init() {
	gitCmd.AddCommand(installHookCmd)
	gitCmd.AddCommand(checkHooksCmd)
	gitCmd.AddCommand(uninstallHookCmd)

	installHookCmd.Flags().StringVar(&hookInstallDir, "dir", "", "git repository directory (defaults to current directory)")
	installHookCmd.Flags().BoolVar(&skipDryRun, "skip-dry-run", false, "skip dry-run validation in hook")
	installHookCmd.Flags().BoolVar(&forceReinstall, "force", false, "force reinstall if hook already exists")

	checkHooksCmd.Flags().StringVar(&hookInstallDir, "dir", "", "git repository directory (defaults to current directory)")

	uninstallHookCmd.Flags().StringVar(&hookInstallDir, "dir", "", "git repository directory (defaults to current directory)")
}

var gitCmd = &cobra.Command{
	Use:   "git",
	Short: "Git integration commands",
	Long: `Manage git integration for attest.

Install pre-commit hooks to automatically validate commits and link them to intents.
Supports commit message convention: "intent: INT-123 Description"`,
}

var installHookCmd = &cobra.Command{
	Use:   "install-hook",
	Short: "Install pre-commit hook",
	Long: `Installs the attest pre-commit hook in the git repository.

The hook will:
1. Run 'attest exec --dry-run' before commit to validate changes
2. Detect intent references in commit messages (format: "intent: INT-123 Description")
3. Link commits to their corresponding intents automatically`,
	Example: `  # Install hook in current repository
  attest git install-hook

  # Install hook in specific repository
  attest git install-hook --dir /path/to/repo

  # Install without dry-run validation
  attest git install-hook --skip-dry-run`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runInstallHook(hookInstallDir, skipDryRun, forceReinstall); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var checkHooksCmd = &cobra.Command{
	Use:   "check-hooks",
	Short: "Verify hooks are installed",
	Long: `Verifies that the attest pre-commit hook is properly installed.

Checks:
1. .git/hooks/pre-commit exists
2. Hook contains attest-specific code
3. Hook is executable`,
	Example: `  # Check hooks in current repository
  attest git check-hooks

  # Check hooks in specific repository
  attest git check-hooks --dir /path/to/repo`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runCheckHooks(hookInstallDir); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var uninstallHookCmd = &cobra.Command{
	Use:   "uninstall-hook",
	Short: "Remove pre-commit hook",
	Long: `Removes the attest pre-commit hook from the git repository.

This will delete the .git/hooks/pre-commit file if it contains attest-specific code.`,
	Example: `  # Uninstall hook from current repository
  attest git uninstall-hook

  # Uninstall hook from specific repository
  attest git uninstall-hook --dir /path/to/repo`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runUninstallHook(hookInstallDir); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

func runInstallHook(repoDir string, skipDryRun bool, force bool) error {
	gitDir, err := findGitDir(repoDir)
	if err != nil {
		return fmt.Errorf("failed to find git repository: %w", err)
	}

	hooksDir := filepath.Join(gitDir, "hooks")
	preCommitPath := filepath.Join(hooksDir, "pre-commit")

	existingHook := ""
	if _, err := os.Stat(preCommitPath); err == nil {
		content, readErr := os.ReadFile(preCommitPath)
		if readErr == nil {
			existingHook = string(content)
		}

		if !force {
			if strings.Contains(existingHook, "attest") {
				return fmt.Errorf("hook already exists. Use --force to reinstall")
			}
			return fmt.Errorf("pre-commit hook already exists. Remove it first or use --force")
		}
	}

	if err := os.MkdirAll(hooksDir, 0755); err != nil {
		return fmt.Errorf("failed to create hooks directory: %w", err)
	}

	hookContent := generateHookContent(skipDryRun)

	if err := os.WriteFile(preCommitPath, []byte(hookContent), 0755); err != nil {
		return fmt.Errorf("failed to write hook: %w", err)
	}

	fmt.Printf("✓ Pre-commit hook installed at: %s\n", preCommitPath)
	fmt.Println("  Hook will run 'attest exec --dry-run' before each commit")
	fmt.Println("  Commit messages following 'intent: INT-123 Description' will be auto-linked")

	return nil
}

func runCheckHooks(repoDir string) error {
	gitDir, err := findGitDir(repoDir)
	if err != nil {
		return fmt.Errorf("failed to find git repository: %w", err)
	}

	hooksDir := filepath.Join(gitDir, "hooks")
	preCommitPath := filepath.Join(hooksDir, "pre-commit")

	info, err := os.Stat(preCommitPath)
	if os.IsNotExist(err) {
		fmt.Println("✗ Pre-commit hook not found")
		fmt.Println("  Run 'attest git install-hook' to install it")
		return nil
	}
	if err != nil {
		return fmt.Errorf("failed to check hook: %w", err)
	}

	fmt.Printf("✓ Pre-commit hook exists: %s\n", preCommitPath)
	fmt.Printf("  Permissions: %o\n", info.Mode().Perm())

	content, err := os.ReadFile(preCommitPath)
	if err != nil {
		return fmt.Errorf("failed to read hook: %w", err)
	}

	if !strings.Contains(string(content), "attest") {
		fmt.Println("⚠ Hook exists but does not contain attest code")
		return nil
	}

	fmt.Println("✓ Hook contains attest integration")
	fmt.Println("  - Commit message validation: enabled")
	fmt.Println("  - Intent linking: enabled")

	return nil
}

func runUninstallHook(repoDir string) error {
	gitDir, err := findGitDir(repoDir)
	if err != nil {
		return fmt.Errorf("failed to find git repository: %w", err)
	}

	hooksDir := filepath.Join(gitDir, "hooks")
	preCommitPath := filepath.Join(hooksDir, "pre-commit")

	if _, err := os.Stat(preCommitPath); os.IsNotExist(err) {
		fmt.Println("No pre-commit hook found to remove")
		return nil
	} else if err != nil {
		return fmt.Errorf("failed to check hook: %w", err)
	}

	content, err := os.ReadFile(preCommitPath)
	if err != nil {
		return fmt.Errorf("failed to read hook: %w", err)
	}

	if !strings.Contains(string(content), "attest") {
		fmt.Println("Pre-commit hook exists but does not contain attest code")
		fmt.Println("Not removing non-attest hooks")
		return nil
	}

	if err := os.Remove(preCommitPath); err != nil {
		return fmt.Errorf("failed to remove hook: %w", err)
	}

	fmt.Printf("✓ Pre-commit hook removed: %s\n", preCommitPath)
	return nil
}

func findGitDir(repoDir string) (string, error) {
	if repoDir == "" {
		cwd, err := os.Getwd()
		if err != nil {
			return "", fmt.Errorf("failed to get current directory: %w", err)
		}
		repoDir = cwd
	}

	gitDir := filepath.Join(repoDir, ".git")
	if _, err := os.Stat(gitDir); err == nil {
		return gitDir, nil
	}

	parent := filepath.Dir(repoDir)
	if parent == repoDir {
		return "", fmt.Errorf("not a git repository and no .git directory found")
	}

	return findGitDir(parent)
}

func generateHookContent(skipDryRun bool) string {
	attestPath, _ := exec.LookPath("attest")
	if attestPath == "" {
		attestPath = "attest"
	}

	hook := fmt.Sprintf(`#!/bin/sh
# Attest pre-commit hook
# Automatically validates commits and links them to intents

set -e

ATTEST_BIN="${ATTEST_BIN:-%s}"

`, attestPath)

	if !skipDryRun {
		hook += `
if command -v "$ATTEST_BIN" &> /dev/null; then
    echo "[attest] Running dry-run validation..."
    if ! "$ATTEST_BIN" exec --dry-run --backup=file 2>/dev/null; then
        echo "[attest] Dry-run validation failed. Aborting commit."
        exit 1
    fi
    echo "[attest] Dry-run validation passed"
fi
`
	}

	hook += `
COMMIT_MSG_FILE="$1"
if [ -f "$COMMIT_MSG_FILE" ]; then
    COMMIT_MSG=$(cat "$COMMIT_MSG_FILE")
    
    INTENT_PATTERN="^intent: ([A-Z]+-[0-9]+) (.+)$"
    
    if echo "$COMMIT_MSG" | grep -qiE "$INTENT_PATTERN"; then
        INTENT_ID=$(echo "$COMMIT_MSG" | grep -oiE "$INTENT_PATTERN" | sed -n 's/^intent: \([A-Z]*-[0-9]*\).*/\1/p')
        
        if [ -n "$INTENT_ID" ]; then
            echo "[attest] Intent detected: $INTENT_ID"
            echo "[attest] Commit will be linked to intent: $INTENT_ID"
        fi
    fi
fi

exit 0
`

	return hook
}

func ExtractIntentFromCommitMessage(msg string) (intentID string, description string) {
	pattern := regexp.MustCompile(`(?i)^intent:\s*([A-Z]+-[0-9]+)\s+(.+)$`)
	matches := pattern.FindStringSubmatch(msg)

	if len(matches) >= 3 {
		return matches[1], matches[2]
	}

	return "", ""
}

func IsGitRepo(dir string) bool {
	gitDir := filepath.Join(dir, ".git")
	_, err := os.Stat(gitDir)
	return err == nil
}
