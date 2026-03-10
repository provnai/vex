// cmd/cost.go - CLI commands for cost management

package cmd

import (
	"context"
	"fmt"
	"strings"

	"github.com/fatih/color"
	"github.com/provnai/attest/internal/db"
	"github.com/provnai/attest/pkg/cost"
	"github.com/spf13/cobra"
)

var (
	costDailyFlag     float64
	costWeeklyFlag    float64
	costMonthlyFlag   float64
	costHardStopFlag  bool
	costWarnThreshold float64
)

// costCmd represents the cost command
var costCmd = &cobra.Command{
	Use:   "cost",
	Short: "Manage API cost tracking and budgets",
	Long: `Track and control API spending across agent runs.
	
Set budget limits to prevent unexpected costs and monitor usage
across OpenAI, Anthropic, and other AI providers.`,
}

// costStatusCmd shows current spending status
var costStatusCmd = &cobra.Command{
	Use:   "status",
	Short: "Show current spending and budget status",
	RunE: func(cmd *cobra.Command, args []string) error {
		database, err := db.Open()
		if err != nil {
			return fmt.Errorf("failed to open database: %w", err)
		}
		defer database.Close()

		limiter := cost.NewLimiter(database)
		status, err := limiter.GetFullStatus(context.Background())
		if err != nil {
			return fmt.Errorf("failed to get status: %w", err)
		}

		printStatus(status)
		return nil
	},
}

// costSetCmd sets budget limits
var costSetCmd = &cobra.Command{
	Use:   "set",
	Short: "Set budget limits",
	Long: `Set daily, weekly, or monthly budget limits.
	
Examples:
  attest cost set --daily 50        # Set $50/day limit
  attest cost set --weekly 200      # Set $200/week limit
  attest cost set --monthly 500     # Set $500/month limit
  attest cost set --daily 10 --weekly 50 --monthly 200  # Set all limits`,
	RunE: func(cmd *cobra.Command, args []string) error {
		database, err := db.Open()
		if err != nil {
			return fmt.Errorf("failed to open database: %w", err)
		}
		defer database.Close()

		limiter := cost.NewLimiter(database)
		config, err := limiter.GetBudgetConfig(context.Background())
		if err != nil {
			return fmt.Errorf("failed to get config: %w", err)
		}

		// Update only flags that were explicitly set
		if cmd.Flags().Changed("daily") {
			config.DailyLimit = costDailyFlag
		}
		if cmd.Flags().Changed("weekly") {
			config.WeeklyLimit = costWeeklyFlag
		}
		if cmd.Flags().Changed("monthly") {
			config.MonthlyLimit = costMonthlyFlag
		}
		if cmd.Flags().Changed("hard-stop") {
			config.HardStop = costHardStopFlag
		}
		if cmd.Flags().Changed("warn-threshold") {
			if costWarnThreshold < 0 || costWarnThreshold > 1 {
				return fmt.Errorf("warn-threshold must be between 0.0 and 1.0")
			}
			config.WarnThreshold = costWarnThreshold
		}

		_, err = limiter.SetBudgetConfig(context.Background(), config)
		if err != nil {
			return fmt.Errorf("failed to set budget: %w", err)
		}

		fmt.Println(color.GreenString("✓ Budget settings updated"))
		fmt.Println()

		// Show current config
		status, _ := limiter.GetFullStatus(context.Background())
		printStatus(status)

		return nil
	},
}

// costReportCmd shows spending history
var costReportCmd = &cobra.Command{
	Use:   "report",
	Short: "Show detailed spending report",
	Long: `Display detailed spending reports by period.
	
Shows breakdown by model, provider, and individual API calls.`,
	RunE: func(cmd *cobra.Command, args []string) error {
		database, err := db.Open()
		if err != nil {
			return fmt.Errorf("failed to open database: %w", err)
		}
		defer database.Close()

		tracker := cost.NewTracker(database)
		limiter := cost.NewLimiter(database)
		config, _ := limiter.GetBudgetConfig(context.Background())

		fmt.Println(color.CyanString("═══════════════════════════════════════════"))
		fmt.Println(color.CyanString("          COST SPENDING REPORT              "))
		fmt.Println(color.CyanString("═══════════════════════════════════════════"))
		fmt.Println()

		// Daily Report
		dailyReport, err := tracker.GetReport(context.Background(), cost.PeriodDaily)
		if err != nil {
			return err
		}
		dailyReport.BudgetLimit = config.DailyLimit
		dailyReport.Percentage = dailyReport.TotalSpent / config.DailyLimit
		printPeriodReport("DAILY", dailyReport)

		// Weekly Report
		weeklyReport, err := tracker.GetReport(context.Background(), cost.PeriodWeekly)
		if err != nil {
			return err
		}
		weeklyReport.BudgetLimit = config.WeeklyLimit
		weeklyReport.Percentage = weeklyReport.TotalSpent / config.WeeklyLimit
		printPeriodReport("WEEKLY", weeklyReport)

		// Monthly Report
		monthlyReport, err := tracker.GetReport(context.Background(), cost.PeriodMonthly)
		if err != nil {
			return err
		}
		monthlyReport.BudgetLimit = config.MonthlyLimit
		monthlyReport.Percentage = monthlyReport.TotalSpent / config.MonthlyLimit
		printPeriodReport("MONTHLY", monthlyReport)

		return nil
	},
}

// costResetCmd resets all cost data (with confirmation)
var costResetCmd = &cobra.Command{
	Use:   "reset",
	Short: "Reset all cost tracking data",
	Long:  `WARNING: This will delete all cost tracking history!`,
	RunE: func(cmd *cobra.Command, args []string) error {
		fmt.Print(color.RedString("WARNING: This will delete ALL cost tracking data!\n"))
		fmt.Print("Are you sure? Type 'yes' to confirm: ")

		var response string
		if _, err := fmt.Scanln(&response); err != nil {
			return fmt.Errorf("failed to read response: %w", err)
		}

		if strings.ToLower(response) != "yes" {
			fmt.Println("Aborted.")
			return nil
		}

		database, err := db.Open()
		if err != nil {
			return fmt.Errorf("failed to open database: %w", err)
		}
		defer database.Close()

		tracker := cost.NewTracker(database)
		err = tracker.ResetCosts(context.Background())
		if err != nil {
			return fmt.Errorf("failed to reset costs: %w", err)
		}

		fmt.Println(color.GreenString("✓ All cost data has been reset"))
		return nil
	},
}

func printStatus(status *cost.BudgetStatus) {
	cyan := color.New(color.FgCyan, color.Bold)
	green := color.New(color.FgGreen)
	yellow := color.New(color.FgYellow)
	red := color.New(color.FgRed, color.Bold)

	cyan.Println("═══════════════════════════════════════════")
	cyan.Println("          COST BUDGET STATUS               ")
	cyan.Println("═══════════════════════════════════════════")
	fmt.Println()

	// Daily
	printBudgetLine("Daily", status.Spending.Daily, status.Config.DailyLimit, status.Config.WarnThreshold, green, yellow, red)

	// Weekly
	printBudgetLine("Weekly", status.Spending.Weekly, status.Config.WeeklyLimit, status.Config.WarnThreshold, green, yellow, red)

	// Monthly
	printBudgetLine("Monthly", status.Spending.Monthly, status.Config.MonthlyLimit, status.Config.WarnThreshold, green, yellow, red)

	fmt.Println()
	fmt.Printf("Hard Stop: %s\n", formatBool(status.Config.HardStop))
	fmt.Printf("Warning Threshold: %.0f%%\n", status.Config.WarnThreshold*100)
	fmt.Printf("Last Updated: %s\n", status.Config.UpdatedAt.Format("2006-01-02 15:04:05"))
	fmt.Printf("As of: %s\n", status.Spending.AsOf.Format("2006-01-02 15:04:05"))

	if len(status.Warnings) > 0 {
		fmt.Println()
		red.Println("⚠ WARNINGS:")
		for _, warning := range status.Warnings {
			red.Println("  •", warning)
		}
	}
}

func printBudgetLine(period string, spent, limit, warnThreshold float64, green, yellow, red *color.Color) {
	percentage := 0.0
	if limit > 0 {
		percentage = spent / limit
	}

	statusColor := green
	if percentage >= 1.0 {
		statusColor = red
	} else if percentage >= warnThreshold {
		statusColor = yellow
	}

	bar := renderProgressBar(percentage, 30)

	fmt.Printf("%-8s: %s %s/%s (%.1f%%)\n",
		period,
		statusColor.Sprint(bar),
		cost.FormatCurrency(spent),
		cost.FormatCurrency(limit),
		percentage*100,
	)
}

func renderProgressBar(percentage float64, width int) string {
	if percentage > 1.0 {
		percentage = 1.0
	}

	filled := int(percentage * float64(width))
	if filled > width {
		filled = width
	}

	empty := width - filled

	bar := strings.Repeat("█", filled) + strings.Repeat("░", empty)
	return bar
}

func formatBool(b bool) string {
	if b {
		return color.GreenString("Enabled")
	}
	return color.YellowString("Disabled")
}

func printPeriodReport(periodName string, report *cost.SpendingReport) {
	cyan := color.New(color.FgCyan, color.Bold)

	cyan.Printf("\n%s REPORT (%s)\n", periodName, report.StartDate.Format("2006-01-02"))
	fmt.Println(strings.Repeat("─", 50))

	status := "✓"
	colorFunc := color.GreenString
	if report.Percentage >= 1.0 {
		status = "✗"
		colorFunc = color.RedString
	} else if report.Percentage >= 0.8 {
		status = "⚠"
		colorFunc = color.YellowString
	}

	fmt.Printf("Total Spent:  %s %s\n", colorFunc(status), colorFunc(cost.FormatCurrency(report.TotalSpent)))
	fmt.Printf("Budget Limit: %s\n", cost.FormatCurrency(report.BudgetLimit))
	fmt.Printf("Percentage:   %.1f%%\n", report.Percentage*100)

	if len(report.ByModel) > 0 {
		fmt.Println("\nBy Model:")
		for model, costAmount := range report.ByModel {
			fmt.Printf("  • %-25s %s\n", model, cost.FormatCurrency(costAmount))
		}
	}

	if len(report.ByProvider) > 0 {
		fmt.Println("\nBy Provider:")
		for provider, costAmount := range report.ByProvider {
			fmt.Printf("  • %-25s %s\n", provider, cost.FormatCurrency(costAmount))
		}
	}

	if len(report.Entries) > 0 && periodName == "DAILY" {
		fmt.Println("\nRecent Calls:")
		for i, entry := range report.Entries {
			if i >= 5 {
				fmt.Printf("  ... and %d more\n", len(report.Entries)-5)
				break
			}
			fmt.Printf("  • %s | %-20s | %s | %d→%d tokens\n",
				entry.CreatedAt.Format("15:04"),
				entry.Model,
				cost.FormatCurrency(entry.TotalCost),
				entry.InputTokens,
				entry.OutputTokens,
			)
		}
	}
}

func init() {
	rootCmd.AddCommand(costCmd)
	costCmd.AddCommand(costStatusCmd)
	costCmd.AddCommand(costSetCmd)
	costCmd.AddCommand(costReportCmd)
	costCmd.AddCommand(costResetCmd)

	// Flags for cost set
	costSetCmd.Flags().Float64Var(&costDailyFlag, "daily", 0, "Daily budget limit in USD")
	costSetCmd.Flags().Float64Var(&costWeeklyFlag, "weekly", 0, "Weekly budget limit in USD")
	costSetCmd.Flags().Float64Var(&costMonthlyFlag, "monthly", 0, "Monthly budget limit in USD")
	costSetCmd.Flags().BoolVar(&costHardStopFlag, "hard-stop", true, "Stop execution when budget exceeded")
	costSetCmd.Flags().Float64Var(&costWarnThreshold, "warn-threshold", 0.8, "Warning threshold (0.0-1.0)")
}
