package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"

	"github.com/provnai/attest/pkg/intent"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
)

var (
	intentTicket   string
	intentDesc     string
	intentConstr   string
	intentCriteria string
	intentStatus   string
)

func init() {
	intentCmd.AddCommand(intentCreateCmd)
	intentCmd.AddCommand(intentListCmd)
	intentCmd.AddCommand(intentShowCmd)
	intentCmd.AddCommand(intentLinkCmd)

	intentCreateCmd.Flags().StringVarP(&intentTicket, "ticket", "i", "", "ticket/issue ID (e.g., WEB-123)")
	intentCreateCmd.Flags().StringVarP(&intentDesc, "description", "d", "", "description of the intent")
	intentCreateCmd.Flags().StringVarP(&intentConstr, "constraints", "c", "", "comma-separated constraints")
	intentCreateCmd.Flags().StringVarP(&intentCriteria, "acceptance", "a", "", "comma-separated acceptance criteria")
	intentCreateCmd.Flags().StringVarP(&intentStatus, "status", "s", "open", "initial status (open, in_progress)")

	intentListCmd.Flags().StringVarP(&intentStatus, "status", "s", "", "filter by status (open, completed, failed)")

	intentShowCmd.Flags().BoolVar(&jsonOutput, "json", false, "output as JSON")
}

var intentCmd = &cobra.Command{
	Use:   "intent",
	Short: "Manage agent intents",
	Long:  `Create and track the goals and intentions behind agent actions.`,
}

var intentCreateCmd = &cobra.Command{
	Use:   "create [goal]",
	Short: "Create a new intent",
	Long: `Create a new intent record with goal, constraints, and acceptance criteria.
Links to tickets for traceability.`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		goal := args[0]

		constraints := []string{}
		if intentConstr != "" {
			constraints = strings.Split(intentConstr, ",")
		}
		criteria := []string{}
		if intentCriteria != "" {
			criteria = strings.Split(intentCriteria, ",")
		}

		i := intent.CreateIntent(goal, intentDesc, intentTicket, constraints, criteria)

		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
		defer db.Close()

		store := storage.NewIntentStore(db)
		if err := store.SaveIntent(i); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}

		fmt.Printf("Intent created successfully!\n")
		fmt.Printf("ID:       %s\n", i.ID)
		fmt.Printf("Goal:     %s\n", i.Goal)
		fmt.Printf("Ticket:   %s\n", i.TicketID)
		fmt.Printf("Status:   %s\n", i.Status)
		fmt.Printf("Created:  %s\n", i.CreatedAt.Format("2006-01-02 15:04:05"))
	},
}

var intentListCmd = &cobra.Command{
	Use:   "list",
	Short: "List intents",
	Long:  `List all intents with optional filtering by status.`,
	Run: func(cmd *cobra.Command, args []string) {
		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
		defer db.Close()

		store := storage.NewIntentStore(db)
		intents, err := store.ListIntents(intentStatus)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}

		if jsonOutput {
			data, _ := json.MarshalIndent(intents, "", "  ")
			fmt.Println(string(data))
			return
		}

		fmt.Printf("%-22s %-30s %-12s %-15s\n", "ID", "GOAL", "STATUS", "CREATED")
		fmt.Printf("%-22s %-30s %-12s %-15s\n", "----------------------", "------------------------------", "------------", "---------------")
		for _, i := range intents {
			goal := i.Goal
			if len(goal) > 28 {
				goal = goal[:28] + "..."
			}
			fmt.Printf("%-22s %-30s %-12s %s\n", i.ID, goal, i.Status, i.CreatedAt.Format("2006-01-02"))
		}
		fmt.Printf("\nTotal: %d intents\n", len(intents))
	},
}

var intentShowCmd = &cobra.Command{
	Use:   "show [id]",
	Short: "Show intent details",
	Long:  `Show detailed information about a specific intent with linked actions.`,
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		id := args[0]

		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
		defer db.Close()

		store := storage.NewIntentStore(db)
		i, err := store.GetIntent(id)
		if err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}

		if jsonOutput {
			data, _ := json.MarshalIndent(i, "", "  ")
			fmt.Println(string(data))
			return
		}

		fmt.Printf("Intent Details\n")
		fmt.Printf("==============\n")
		fmt.Printf("ID:       %s\n", i.ID)
		fmt.Printf("Goal:     %s\n", i.Goal)
		fmt.Printf("Desc:     %s\n", i.Description)
		fmt.Printf("Ticket:   %s\n", i.TicketID)
		fmt.Printf("Status:   %s\n", i.Status)
		fmt.Printf("Created:  %s\n", i.CreatedAt.Format("2006-01-02 15:04:05"))
		if i.ClosedAt != nil {
			fmt.Printf("Closed:   %s\n", i.ClosedAt.Format("2006-01-02 15:04:05"))
		}
		if len(i.Constraints) > 0 {
			fmt.Printf("Constraints: %v\n", i.Constraints)
		}
		if len(i.AcceptanceCriteria) > 0 {
			fmt.Printf("Acceptance:  %v\n", i.AcceptanceCriteria)
		}
	},
}

var intentLinkCmd = &cobra.Command{
	Use:   "link [intent-id] [attestation-id]",
	Short: "Link an action to an intent",
	Long:  `Associate an attestation with an intent record.`,
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		// For now, just print that it would link
		fmt.Printf("Would link intent '%s' to attestation '%s'\n", args[0], args[1])
		fmt.Printf("(Full linking implementation coming soon)\n")
	},
}
