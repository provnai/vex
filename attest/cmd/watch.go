package cmd

import (
	"fmt"
	"os"
	"os/exec"
	"path/filepath"
	"strings"
	"time"

	"github.com/provnai/attest/pkg/instrument"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
)

var watchCmd = &cobra.Command{
	Use:   "watch",
	Short: "Watch and auto-capture agent actions",
	Long: `Watch a command and automatically capture all agent actions.
No code changes required - just prefix your command with 'attest watch --'.

Examples:
  attest watch -- python my_agent.py
  attest watch --intent int:abc123 -- node agent.js
  attest watch -- python langchain_agent.py`,
	Run: func(cmd *cobra.Command, args []string) {
		runWatch(args)
	},
}

var (
	watchIntent   string
	watchAutoLink bool
	watchJSON     bool
	watchLLM      bool
	watchFS       bool
	watchNet      bool
)

func init() {
	watchCmd.Flags().StringVar(&watchIntent, "intent", "", "link captured actions to intent ID")
	watchCmd.Flags().BoolVar(&watchAutoLink, "auto-link", false, "automatically link to most recent open intent")
	watchCmd.Flags().BoolVar(&watchLLM, "llm", true, "capture LLM calls")
	watchCmd.Flags().BoolVar(&watchFS, "fs", true, "capture file operations")
	watchCmd.Flags().BoolVar(&watchNet, "net", true, "capture network requests")
	watchCmd.Flags().BoolVar(&watchJSON, "json", false, "output as JSON")

	rootCmd.AddCommand(watchCmd)
}

func runWatch(args []string) {
	if len(args) == 0 {
		fmt.Println("Error: command required")
		fmt.Println("Usage: attest watch -- <command>")
		return
	}

	sepIndex := -1
	for i, arg := range args {
		if arg == "--" {
			sepIndex = i
			break
		}
	}

	var command string
	var cmdArgs []string

	if sepIndex >= 0 {
		command = args[0]
		cmdArgs = args[1:]
	} else {
		command = args[0]
		cmdArgs = args[1:]
	}

	if watchAutoLink && watchIntent == "" {
		recentIntent := findMostRecentOpenIntent()
		if recentIntent != "" {
			watchIntent = recentIntent
			fmt.Printf("✓ Auto-linked to recent intent: %s\n", watchIntent)
		} else {
			fmt.Println("⚠ No open intents found. Run 'attest intent create' first.")
		}
	}

	tracer := instrument.NewTracer()
	sessionID := fmt.Sprintf("sess:%d", time.Now().UnixNano())
	tracer.Start(sessionID, watchLLM, watchFS, watchNet)

	fmt.Printf("✓ Starting watch session: %s\n", sessionID)
	fmt.Printf("✓ Watching: %s %s\n", command, strings.Join(cmdArgs, " "))

	if watchIntent != "" {
		fmt.Printf("✓ Linked to intent: %s\n", watchIntent)
	}

	wd, _ := os.Getwd()
	capturedFiles := make(map[string]bool)

	cmd := exec.Command(command, cmdArgs...)
	cmd.Stdout = os.Stdout
	cmd.Stderr = os.Stderr
	cmd.Stdin = os.Stdin

	err := cmd.Run()

	scanDirectoryForChanges(wd, capturedFiles, tracer)

	tracer.Stop()

	summary := tracer.GetSessionSummary()
	actions := tracer.GetActions()

	fmt.Printf("\n=== Watch Session Complete ===\n")
	fmt.Printf("Session: %s\n", sessionID)
	fmt.Printf("Actions captured: %d\n", len(actions))

	if llm, ok := summary["by_type"].(map[string]int)["llm"]; ok && llm > 0 {
		fmt.Printf("  - LLM calls: %d\n", llm)
	}
	if fs, ok := summary["by_type"].(map[string]int)["file"]; ok && fs > 0 {
		fmt.Printf("  - File ops: %d\n", fs)
	}
	if net, ok := summary["by_type"].(map[string]int)["network"]; ok && net > 0 {
		fmt.Printf("  - Network: %d\n", net)
	}
	if exec, ok := summary["by_type"].(map[string]int)["exec"]; ok && exec > 0 {
		fmt.Printf("  - Executions: %d\n", exec)
	}

	if watchIntent != "" {
		fmt.Printf("Intent: %s\n", watchIntent)
	}

	if err != nil {
		fmt.Printf("\nCommand exited with error: %v\n", err)
	} else {
		fmt.Printf("\nCommand completed successfully\n")
	}

	if watchJSON {
		fmt.Printf("\n=== JSON Output ===\n")
		fmt.Printf("{\"session_id\": \"%s\", \"actions\": %d}\n", sessionID, len(actions))
	}
}

func scanDirectoryForChanges(rootDir string, capturedFiles map[string]bool, tracer *instrument.Tracer) {
	if !watchFS {
		return
	}

	if err := filepath.Walk(rootDir, func(path string, info os.FileInfo, err error) error {
		if err != nil {
			return nil
		}

		relPath, _ := filepath.Rel(rootDir, path)

		if strings.HasPrefix(relPath, ".git") ||
			strings.HasPrefix(relPath, ".attest") ||
			strings.HasPrefix(relPath, "node_modules") ||
			strings.HasPrefix(relPath, "__pycache__") ||
			strings.Contains(relPath, ".pyc") ||
			relPath == "attest" || relPath == "attest.exe" {
			return nil
		}

		if !info.IsDir() && isRelevantFile(relPath) {
			// Limit file capture to 1MB to avoid memory bombs
			if info.Size() < 1024*1024 {
				content, err := os.ReadFile(path)
				if err == nil {
					op := "modified"
					if !capturedFiles[relPath] {
						capturedFiles[relPath] = true
						op = "created"
					}

					tracer.TraceFileOperation(op, relPath, string(content))
				}
			}
		}

		return nil
	}); err != nil {
		fmt.Printf("Warning: failed to scan directory: %v\n", err)
	}
}

func isRelevantFile(path string) bool {
	relevantExts := map[string]bool{
		".py":   true,
		".js":   true,
		".ts":   true,
		".json": true,
		".yaml": true,
		".yml":  true,
		".txt":  true,
		".md":   true,
		".sh":   true,
		".bat":  true,
		".sql":  true,
	}

	ext := strings.ToLower(filepath.Ext(path))
	return relevantExts[ext] || strings.Contains(path, "requirements.txt") || strings.Contains(path, "package.json")
}

func findMostRecentOpenIntent() string {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return ""
	}
	defer db.Close()

	store := storage.NewIntentStore(db)
	intents, err := store.ListIntents("open")
	if err != nil {
		return ""
	}

	if len(intents) == 0 {
		return ""
	}

	return intents[0].ID
}
