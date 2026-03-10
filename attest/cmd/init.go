package cmd

import (
	"fmt"
	"os"
	"time"

	"github.com/fatih/color"
	"github.com/provnai/attest/internal/setup"
	"github.com/spf13/cobra"
)

var (
	framework  string
	autoDetect bool
	skipGit    bool
	skipCI     bool
)

var initCmd = &cobra.Command{
	Use:   "init",
	Short: "Initialize Attest in your project",
	Long: `Initialize Attest with one command. Auto-detects your framework and sets up everything needed.

Examples:
  attest init --langchain      # Setup for LangChain project
  attest init --autogen        # Setup for AutoGen project  
  attest init --crewai         # Setup for CrewAI project
  attest init --auto-detect    # Auto-detect framework from project files`,
	Run: func(cmd *cobra.Command, args []string) {
		startTime := time.Now()

		green := color.New(color.FgGreen).SprintFunc()
		blue := color.New(color.FgBlue).SprintFunc()
		yellow := color.New(color.FgYellow).SprintFunc()

		fmt.Println(blue("🚀 Attest One-Command Integration"))
		fmt.Println()

		// Step 1: Detect framework
		fmt.Println("📋 Step 1: Detecting framework...")
		detectedFramework := framework
		if autoDetect || framework == "" {
			detectedFramework = setup.DetectFramework()
			if detectedFramework != "" {
				fmt.Printf("   %s Detected: %s\n", green("✓"), detectedFramework)
			} else {
				fmt.Printf("   %s No framework detected. Use --framework flag.\n", yellow("⚠"))
				os.Exit(1)
			}
		} else {
			fmt.Printf("   %s Using: %s\n", green("✓"), framework)
		}

		// Step 2: Create config file
		fmt.Println("\n⚙️  Step 2: Creating configuration...")
		configPath, err := setup.CreateConfig(detectedFramework)
		if err != nil {
			fmt.Printf("   ✗ Failed to create config: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("   %s Created: %s\n", green("✓"), configPath)

		// Step 3: Install framework-specific hooks
		fmt.Println("\n🔌 Step 3: Installing framework hooks...")
		hookPath, err := setup.InstallFrameworkHooks(detectedFramework)
		if err != nil {
			fmt.Printf("   ✗ Failed to install hooks: %v\n", err)
			os.Exit(1)
		}
		fmt.Printf("   %s Installed: %s\n", green("✓"), hookPath)

		// Step 4: Setup git integration
		if !skipGit {
			fmt.Println("\n📦 Step 4: Configuring git integration...")
			if err := setup.SetupGitHooks(); err != nil {
				fmt.Printf("   ⚠ Git hooks setup skipped: %v\n", err)
			} else {
				fmt.Printf("   %s Git hooks configured\n", green("✓"))
			}
		}

		// Step 5: Create CI templates
		if !skipCI {
			fmt.Println("\n🔄 Step 5: Setting up CI templates...")
			ciPath, err := setup.SetupCITemplates(detectedFramework)
			if err != nil {
				fmt.Printf("   ⚠ CI setup skipped: %v\n", err)
			} else {
				fmt.Printf("   %s CI template: %s\n", green("✓"), ciPath)
			}
		}

		// Summary
		duration := time.Since(startTime)
		fmt.Println()
		fmt.Println(green("✅ Attest setup complete!"))
		fmt.Printf("   Framework: %s\n", detectedFramework)
		fmt.Printf("   Config: %s\n", configPath)
		fmt.Printf("   Time: %.1f seconds\n", duration.Seconds())
		fmt.Println()
		fmt.Println(blue("Next steps:"))
		fmt.Println("   1. Review attest.yaml configuration")
		fmt.Println("   2. Import the callback handler in your code")
		fmt.Println("   3. Run 'attest validate' to test your setup")
		fmt.Println()
	},
}

func init() {
	rootCmd.AddCommand(initCmd)

	initCmd.Flags().StringVar(&framework, "framework", "", "Framework to use (langchain, autogen, crewai, llamaindex)")
	initCmd.Flags().BoolVar(&autoDetect, "auto-detect", false, "Auto-detect framework from project files")
	initCmd.Flags().BoolVar(&skipGit, "skip-git", false, "Skip git hooks setup")
	initCmd.Flags().BoolVar(&skipCI, "skip-ci", false, "Skip CI template setup")

	// Shorthand flags for frameworks
	initCmd.Flags().Bool("langchain", false, "Setup for LangChain")
	initCmd.Flags().Bool("autogen", false, "Setup for AutoGen")
	initCmd.Flags().Bool("crewai", false, "Setup for CrewAI")
	initCmd.Flags().Bool("llamaindex", false, "Setup for LlamaIndex")

	// Pre-run to handle shorthand flags
	initCmd.PreRun = func(cmd *cobra.Command, args []string) {
		if cmd.Flag("langchain").Changed {
			framework = "langchain"
		}
		if cmd.Flag("autogen").Changed {
			framework = "autogen"
		}
		if cmd.Flag("crewai").Changed {
			framework = "crewai"
		}
		if cmd.Flag("llamaindex").Changed {
			framework = "llamaindex"
		}
	}
}
