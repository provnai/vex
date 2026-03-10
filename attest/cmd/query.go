package cmd

import (
	"fmt"

	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
)

var queryCmd = &cobra.Command{
	Use:   "query",
	Short: "Query attestations and intents",
	Long:  `Query attestations and intents by various criteria.`,
}

func init() {
	queryCmd.AddCommand(queryIntentCmd)
	queryCmd.AddCommand(queryTicketCmd)
	queryCmd.AddCommand(queryProvenanceCmd)
}

var queryIntentCmd = &cobra.Command{
	Use:   "intent [search terms]",
	Short: "Query intents by goal",
	Long:  `Search intents by goal text using partial matching.`,
	Args:  cobra.MinimumNArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		query := args[0]
		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			return err
		}
		defer db.Close()

		rows, err := db.Query(`SELECT id, goal, status FROM intents WHERE goal LIKE ?`, "%"+query+"%")
		if err != nil {
			return err
		}
		defer rows.Close()

		fmt.Printf("Intents matching '%s':\n", query)
		for rows.Next() {
			var id, goal, status string
			if err := rows.Scan(&id, &goal, &status); err != nil {
				fmt.Printf("Warning: failed to scan intent: %v\n", err)
				continue
			}
			fmt.Printf("  %s [%s]: %s\n", id, status, goal)
		}
		return nil
	},
}

var queryTicketCmd = &cobra.Command{
	Use:   "ticket [ticket-id]",
	Short: "Query by ticket/issue ID",
	Long:  `Find intents linked to a specific ticket or issue ID.`,
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		ticketID := args[0]
		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			return err
		}
		defer db.Close()

		rows, err := db.Query(`SELECT id, goal, status FROM intents WHERE ticket_id = ?`, ticketID)
		if err != nil {
			return err
		}
		defer rows.Close()

		fmt.Printf("Intents for Ticket %s:\n", ticketID)
		for rows.Next() {
			var id, goal, status string
			if err := rows.Scan(&id, &goal, &status); err != nil {
				fmt.Printf("Warning: failed to scan intent: %v\n", err)
				continue
			}
			fmt.Printf("  %s [%s]: %s\n", id, status, goal)
		}
		return nil
	},
}

var queryProvenanceCmd = &cobra.Command{
	Use:   "provenance [intent-id]",
	Short: "Show provenance chain",
	Long:  `Show the complete provenance chain (actions) for an intent.`,
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		intentID := args[0]
		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			return err
		}
		defer db.Close()

		rows, err := db.Query(`SELECT id, action_type, action_target, timestamp, signature FROM attestations WHERE intent_id = ? ORDER BY timestamp ASC`, intentID)
		if err != nil {
			return err
		}
		defer rows.Close()

		fmt.Printf("Provenance for %s:\n", intentID)
		for rows.Next() {
			var id, goal, target, ts, sig string
			if err := rows.Scan(&id, &goal, &target, &ts, &sig); err != nil {
				fmt.Printf("Warning: failed to scan attestation: %v\n", err)
				continue
			}
			fmt.Printf("  %s | %s %s (%s)\n    Sig: %s...\n", ts, goal, target, id[:8], sig[:16])
		}
		return nil
	},
}
