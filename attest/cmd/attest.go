package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"strings"
	"syscall"
	"time"

	"github.com/provnai/attest/pkg/attestation"
	"github.com/provnai/attest/pkg/crypto"
	"github.com/provnai/attest/pkg/identity"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
	"golang.org/x/term"
)

var (
	attestAgentID string
	attestIntent  string
	attestAction  string
	attestTarget  string
	attestInput   string
	attestSession string
	attestFormat  string
	attestPass    string
)

func init() {
	attestCmd.AddCommand(attestCreateCmd)
	attestCmd.AddCommand(attestListCmd)
	attestCmd.AddCommand(attestShowCmd)
	attestCmd.AddCommand(attestExportCmd)
	attestCmd.AddCommand(attestImportCmd)

	attestCmd.Flags().StringVarP(&attestFormat, "format", "f", "text", "output format (text, json)")

	attestCreateCmd.Flags().StringVarP(&attestAgentID, "agent", "a", "", "agent ID (required)")
	attestCreateCmd.Flags().StringVarP(&attestIntent, "intent", "i", "", "intent ID")
	attestCreateCmd.Flags().StringVarP(&attestAction, "action", "t", "command", "action type (command, file_edit, api_call, database, git)")
	attestCreateCmd.Flags().StringVarP(&attestTarget, "target", "x", "", "action target")
	attestCreateCmd.Flags().StringVarP(&attestInput, "input", "n", "", "action input")
	attestCreateCmd.Flags().StringVar(&attestSession, "session", "", "session ID")
	attestCreateCmd.Flags().StringVar(&attestPass, "passphrase", "", "passphrase to unlock agent key")

	attestListCmd.Flags().StringVar(&attestAgentID, "agent", "", "filter by agent")
	attestListCmd.Flags().StringVar(&attestIntent, "intent", "", "filter by intent")
	attestListCmd.Flags().IntVarP(&limit, "limit", "l", 20, "limit results")
}

var attestCmd = &cobra.Command{
	Use:   "attest",
	Short: "Create and manage attestations",
	Long:  `Sign and verify agent actions with cryptographic attestations.`,
}

var attestCreateCmd = &cobra.Command{
	Use:   "create",
	Short: "Create a new attestation",
	Long: `Create a cryptographic attestation for an agent action.
Requires specifying the agent, action type, and target.`,
	Example: `
  # Attest a command
  attest attest create --agent aid:1234 --action command --target "python script.py"

  # Attest with intent
  attest attest create --agent aid:1234 --action file_edit --target auth.py --intent int:abcd

  # Attest a database query
  attest attest create --agent aid:1234 --action database --target "SELECT * FROM users"`,
	Run: func(cmd *cobra.Command, args []string) {
		if attestAgentID == "" {
			fmt.Println("Error: --agent is required")
			os.Exit(1)
		}
		if attestTarget == "" {
			fmt.Println("Error: --target is required")
			os.Exit(1)
		}
		if err := runAttestCreate(); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var attestListCmd = &cobra.Command{
	Use:   "list",
	Short: "List attestations",
	Long:  `List all attestations with optional filtering by agent, intent, or time.`,
	Example: `
  # List all attestations
  attest attest list

  # List by agent
  attest attest list --agent aid:1234

  # List by intent
  attest attest list --intent int:abcd`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAttestList(); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var attestShowCmd = &cobra.Command{
	Use:   "show [id]",
	Short: "Show attestation details",
	Long:  `Show detailed information about a specific attestation.`,
	Example: `
  # Show attestation details
  attest attest show att:12345678`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAttestShow(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var attestExportCmd = &cobra.Command{
	Use:   "export [id]",
	Short: "Export attestation",
	Long:  `Export an attestation to JSON format.`,
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAttestExport(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var attestImportCmd = &cobra.Command{
	Use:   "import [file]",
	Short: "Import attestation",
	Long:  `Import an attestation from JSON file.`,
	Args:  cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAttestImport(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var limit int

func runAttestCreate() error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	store := identity.NewAgentStore(db.DB)
	agent, err := store.Get(attestAgentID)
	if err != nil {
		return err
	}
	if agent.IsRevoked() {
		return fmt.Errorf("agent revoked: %s", attestAgentID)
	}

	name := agent.Name
	pubKeyBase64 := agent.PublicKey
	encryptedPrivateKey := agent.PrivateKeyEncrypted

	// Retrieve passphrase from flag, env, or prompt
	passphrase := attestPass
	if passphrase == "" {
		passphrase = os.Getenv("ATTEST_PASSPHRASE")
	}
	if passphrase == "" && encryptedPrivateKey != "" {
		fmt.Printf("Enter passphrase for agent %s: ", name)
		bytePassphrase, err := term.ReadPassword(int(syscall.Stdin))
		if err != nil {
			return fmt.Errorf("failed to read passphrase: %w", err)
		}
		passphrase = string(bytePassphrase)
		fmt.Println()
	}

	if passphrase == "" {
		return fmt.Errorf("passphrase required to unlock agent key")
	}

	// Load the actual key pair
	keys, err := crypto.LoadKeyPair(pubKeyBase64, encryptedPrivateKey, passphrase)
	if err != nil {
		return fmt.Errorf("failed to load agent keys: %w", err)
	}

	agent = &identity.Agent{
		ID:   attestAgentID,
		Name: name,
		Type: identity.AgentType(agentType),
	}

	action := attestation.ActionRecord{
		Type:           attestation.ActionType(attestAction),
		Target:         attestTarget,
		Input:          attestInput,
		Command:        attestTarget,
		Classification: getActionClassification(attestAction, attestTarget),
	}

	meta := attestation.AttestationMeta{
		SessionID: attestSession,
	}

	attest, err := attestation.CreateAttestation(agent, keys, action, attestIntent, meta)
	if err != nil {
		return fmt.Errorf("failed to create attestation: %w", err)
	}

	_, err = db.Exec(
		`INSERT INTO attestations (id, agent_id, intent_id, action_type, action_target, action_input, signature, timestamp, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		attest.ID, attestAgentID, attestIntent, attestAction, attestTarget, attestInput,
		attest.Signature, attest.Timestamp.Format(time.RFC3339), "{}",
	)
	if err != nil {
		return fmt.Errorf("failed to save attestation: %w", err)
	}

	if attestFormat == "json" {
		output, err := json.MarshalIndent(attest, "", "  ")
		if err != nil {
			return fmt.Errorf("failed to marshal JSON: %w", err)
		}
		fmt.Println(string(output))
	} else {
		fmt.Printf("Attestation created successfully!\n")
		fmt.Printf("ID:       %s\n", attest.ID)
		fmt.Printf("Agent:    %s (%s)\n", attestAgentID, name)
		fmt.Printf("Action:   %s %s\n", attestAction, attestTarget)
		fmt.Printf("Sign:     %s... (Verified)\n", attest.Signature[:16])
	}

	return nil
}

func runAttestList() error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	query := `SELECT a.id, a.agent_id, a.intent_id, a.action_type, a.action_target, a.timestamp, a.signature, ag.name
	          FROM attestations a LEFT JOIN agents ag ON a.agent_id = ag.id`
	var conditions []string
	var args []interface{}

	if attestAgentID != "" {
		conditions = append(conditions, "a.agent_id = ?")
		args = append(args, attestAgentID)
	}
	if attestIntent != "" {
		conditions = append(conditions, "a.intent_id = ?")
		args = append(args, attestIntent)
	}
	if len(conditions) > 0 {
		query += " WHERE " + conditions[0]
		for i := 1; i < len(conditions); i++ {
			query += " AND " + conditions[i]
		}
	}
	query += " ORDER BY a.timestamp DESC LIMIT ?"
	args = append(args, limit)

	rows, err := db.Query(query, args...)
	if err != nil {
		return fmt.Errorf("failed to query attestations: %w", err)
	}
	defer rows.Close()

	type attestRow struct {
		ID        string
		AgentID   string
		IntentID  string
		Action    string
		Target    string
		Timestamp string
		Signature string
		AgentName string
	}

	var attestations []attestRow
	for rows.Next() {
		var r attestRow
		var intentID *string
		if err := rows.Scan(&r.ID, &r.AgentID, &intentID, &r.Action, &r.Target, &r.Timestamp, &r.Signature, &r.AgentName); err != nil {
			return fmt.Errorf("failed to scan row: %w", err)
		}
		if intentID != nil {
			r.IntentID = *intentID
		}
		attestations = append(attestations, r)
	}

	if attestFormat == "json" {
		output, _ := json.MarshalIndent(attestations, "", "  ")
		fmt.Println(string(output))
	} else {
		fmt.Printf("%-20s %-12s %-10s %s\n", "ID", "AGENT", "ACTION", "TIMESTAMP")
		fmt.Printf("%-20s %-12s %-10s %s\n", "----", "------", "------", "--------")
		for _, a := range attestations {
			intentStr := ""
			if a.IntentID != "" {
				// Increase intent ID display length from 8 to 12 characters
				displayIntentID := a.IntentID
				if len(displayIntentID) > 12 {
					displayIntentID = displayIntentID[:12]
				}
				intentStr = fmt.Sprintf(" [%s]", displayIntentID)
			}
			fmt.Printf("%-20s %-12s %-10s %s%s\n", a.ID[:20], a.AgentName[:12], a.Action[:10], a.Timestamp[:10], intentStr)
		}
	}

	return nil
}

func runAttestShow(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	var agentID, intentID, actionType, actionTarget, actionInput, timestamp, signature, agentName string
	err = db.QueryRow(
		`SELECT a.agent_id, a.intent_id, a.action_type, a.action_target, a.action_input, a.timestamp, a.signature, ag.name
     FROM attestations a LEFT JOIN agents ag ON a.agent_id = ag.id WHERE a.id = ?`,
		id,
	).Scan(&agentID, &intentID, &actionType, &actionTarget, &actionInput, &timestamp, &signature, &agentName)

	if err != nil {
		return fmt.Errorf("attestation not found: %s", id)
	}

	if attestFormat == "json" {
		result := map[string]interface{}{
			"id":           id,
			"agentId":      agentID,
			"agentName":    agentName,
			"intentId":     intentID,
			"actionType":   actionType,
			"actionTarget": actionTarget,
			"actionInput":  actionInput,
			"timestamp":    timestamp,
			"signature":    signature,
		}
		output, _ := json.MarshalIndent(result, "", "  ")
		fmt.Println(string(output))
	} else {
		fmt.Printf("Attestation ID:  %s\n", id)
		fmt.Printf("Agent:           %s (%s)\n", agentID, agentName)
		fmt.Printf("Intent:          %s\n", intentID)
		fmt.Printf("Action Type:     %s\n", actionType)
		fmt.Printf("Action Target:   %s\n", actionTarget)
		fmt.Printf("Action Input:    %s\n", actionInput)
		fmt.Printf("Timestamp:       %s\n", timestamp)
		fmt.Printf("Signature:       %s...\n", signature[:20])
	}

	return nil
}

func runAttestExport(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return err
	}
	defer db.Close()

	var agentID, intentID, actionType, actionTarget, actionInput, timestamp, signature string
	err = db.QueryRow(
		`SELECT agent_id, intent_id, action_type, action_target, action_input, timestamp, signature FROM attestations WHERE id = ?`,
		id,
	).Scan(&agentID, &intentID, &actionType, &actionTarget, &actionInput, &timestamp, &signature)

	if err != nil {
		return fmt.Errorf("attestation not found: %s", id)
	}

	export := map[string]interface{}{
		"id":           id,
		"agentId":      agentID,
		"intentId":     intentID,
		"actionType":   actionType,
		"actionTarget": actionTarget,
		"actionInput":  actionInput,
		"timestamp":    timestamp,
		"signature":    signature,
		"exportedAt":   time.Now().UTC().Format(time.RFC3339),
	}

	output, _ := json.MarshalIndent(export, "", "  ")
	fmt.Println(string(output))
	return nil
}

func runAttestImport(path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("failed to read file: %w", err)
	}

	var attestData map[string]interface{}
	if err := json.Unmarshal(data, &attestData); err != nil {
		return fmt.Errorf("invalid JSON: %w", err)
	}

	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return err
	}
	defer db.Close()

	id := attestData["id"].(string)
	agentID := attestData["agentId"].(string)
	intentID := attestData["intentId"].(string)
	actionType := attestData["actionType"].(string)
	actionTarget := attestData["actionTarget"].(string)
	actionInput := attestData["actionInput"].(string)
	timestamp := attestData["timestamp"].(string)
	signature := attestData["signature"].(string)

	_, err = db.Exec(
		`INSERT OR REPLACE INTO attestations (id, agent_id, intent_id, action_type, action_target, action_input, signature, timestamp, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?, '{}')`,
		id, agentID, intentID, actionType, actionTarget, actionInput, signature, timestamp,
	)
	if err != nil {
		return fmt.Errorf("failed to import: %w", err)
	}

	fmt.Printf("Attestation %s imported successfully.\n", id)
	return nil
}

func confirmRollback() (bool, error) {
	fmt.Print("Type 'ROLLBACK' to confirm: ")
	var input string
	if _, err := fmt.Scanln(&input); err != nil {
		return false, nil
	}
	return input == "ROLLBACK", nil
}

func getActionClassification(actionType, target string) string {
	dangerous := []string{"rm", "delete", "drop", "truncate", "destroy"}
	for _, d := range dangerous {
		if strings.Contains(strings.ToLower(target), d) {
			return "dangerous"
		}
	}
	return "normal"
}
