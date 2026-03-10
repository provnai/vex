package cmd

import (
	"encoding/json"
	"fmt"
	"os"
	"syscall"
	"time"

	"github.com/provnai/attest/pkg/crypto"
	"github.com/provnai/attest/pkg/identity"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
	"golang.org/x/term"
)

var (
	agentName       string
	agentType       string
	agentFormat     string
	agentMeta       string
	agentPassphrase string
)

func init() {
	agentCmd.PersistentFlags().StringVarP(&agentName, "name", "n", "", "agent name (required)")
	agentCmd.PersistentFlags().StringVarP(&agentType, "type", "t", "generic", "agent type (generic, langchain, autogen, crewai, custom)")
	agentCmd.PersistentFlags().StringVarP(&agentFormat, "format", "f", "text", "output format (text, json)")
	agentCmd.PersistentFlags().StringVar(&agentMeta, "meta", "", "JSON metadata")
	agentCreateCmd.Flags().StringVarP(&agentPassphrase, "passphrase", "p", "", "passphrase to encrypt private key (will prompt if not provided)")

	agentCmd.AddCommand(agentCreateCmd)
	agentCmd.AddCommand(agentListCmd)
	agentCmd.AddCommand(agentShowCmd)
	agentCmd.AddCommand(agentDeleteCmd)
	agentCmd.AddCommand(agentExportCmd)
	agentCmd.AddCommand(agentImportCmd)
}

var agentCmd = &cobra.Command{
	Use:   "agent",
	Short: "Manage agent identities",
	Long:  `Create, list, show, export, import, and manage cryptographic agent identities.`,
}

var agentCreateCmd = &cobra.Command{
	Use:   "create",
	Short: "Create a new agent identity",
	Long: `Create a new agent with cryptographic identity.
Generates Ed25519 keypair and stores securely in the database.`,
	Example: `
  # Create a generic agent
  attest agent create --name "my-agent"

  # Create a LangChain agent with metadata
  attest agent create --name "chatbot" --type langchain --meta '{"model":"gpt-4"}'

  # Create an agent and output as JSON
  attest agent create --name "worker" --type custom --json`,
	Run: func(cmd *cobra.Command, args []string) {
		if agentName == "" {
			fmt.Println("Error: --name is required")
			os.Exit(1)
		}
		if err := runAgentCreate(); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var agentListCmd = &cobra.Command{
	Use:   "list",
	Short: "List all agents",
	Long:  `List all registered agents in the repository.`,
	Example: `
  # List all agents
  attest agent list

  # List as JSON
  attest agent list --json

  # List revoked agents
  attest agent list --all`,
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAgentList(); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var agentShowCmd = &cobra.Command{
	Use:   "show [id]",
	Short: "Show agent details",
	Long:  `Show detailed information about a specific agent including public key and metadata.`,
	Example: `
  # Show agent details
  attest agent show aid:12345678

  # Show as JSON
  attest agent show aid:12345678 --json`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAgentShow(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var agentDeleteCmd = &cobra.Command{
	Use:   "delete [id]",
	Short: "Delete (revoke) an agent",
	Long:  `Revoke an agent identity. The agent can no longer create attestations.`,
	Example: `
  # Revoke an agent
  attest agent delete aid:12345678`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAgentDelete(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var agentExportCmd = &cobra.Command{
	Use:   "export [id]",
	Short: "Export agent public key",
	Long:  `Export an agent's public key in various formats.`,
	Example: `
  # Export public key as PEM
  attest agent export aid:12345678 --pem

  # Export as JSON
  attest agent export aid:12345678 --json`,
	Args: cobra.ExactArgs(1),
	Run: func(cmd *cobra.Command, args []string) {
		if err := runAgentExport(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

var agentImportCmd = &cobra.Command{
	Use:   "import",
	Short: "Import an agent from backup",
	Long:  `Import an agent from a JSON backup file created with 'attest agent export'.`,
	Example: `
  # Import from file
  attest agent import /path/to/agent-backup.json`,
	Run: func(cmd *cobra.Command, args []string) {
		if len(args) == 0 {
			fmt.Println("Error: backup file path required")
			os.Exit(1)
		}
		if err := runAgentImport(args[0]); err != nil {
			fmt.Printf("Error: %v\n", err)
			os.Exit(1)
		}
	},
}

func runAgentCreate() error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	if err := db.Migrate(); err != nil {
		return fmt.Errorf("failed to migrate database: %w", err)
	}

	keys, err := crypto.GenerateEd25519KeyPair()
	if err != nil {
		return fmt.Errorf("failed to generate keys: %w", err)
	}

	meta := identity.AgentMeta{}
	if agentMeta != "" {
		if err := json.Unmarshal([]byte(agentMeta), &meta); err != nil {
			return fmt.Errorf("invalid metadata JSON: %w", err)
		}
	}

	agentTypeEnum := identity.AgentType(agentType)
	agent, err := identity.CreateAgent(agentName, agentTypeEnum, keys, meta)
	if err != nil {
		return fmt.Errorf("failed to create agent: %w", err)
	}

	pubKeyBase64 := keys.PublicKeyBase64()

	// Get passphrase for encryption
	passphrase := agentPassphrase
	if passphrase == "" {
		fmt.Print("Enter passphrase to encrypt private key: ")
		bytePassphrase, err := term.ReadPassword(int(syscall.Stdin))
		if err != nil {
			return fmt.Errorf("failed to read passphrase: %w", err)
		}
		passphrase = string(bytePassphrase)
		fmt.Println()

		if len(passphrase) < 8 {
			return fmt.Errorf("passphrase must be at least 8 characters")
		}
	}

	// Encrypt private key
	encryptedPrivateKey, err := keys.EncryptPrivateKey(passphrase)
	if err != nil {
		return fmt.Errorf("failed to encrypt private key: %w", err)
	}

	store := identity.NewAgentStore(db.DB)
	if err := store.SaveWithEncryptedKey(agent, encryptedPrivateKey); err != nil {
		return err
	}

	if agentFormat == "json" {
		output, _ := json.MarshalIndent(agent, "", "  ")
		fmt.Println(string(output))
	} else {
		fmt.Printf("Agent created successfully!\n")
		fmt.Printf("ID:       %s\n", agent.ID)
		fmt.Printf("Name:     %s\n", agent.Name)
		fmt.Printf("Type:     %s\n", agent.Type)
		fmt.Printf("Created:  %s\n", agent.CreatedAt.Format(time.RFC3339))
		fmt.Printf("\nPublic Key: %s...\n", pubKeyBase64[:32])
	}

	return nil
}

func runAgentList() error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	store := identity.NewAgentStore(db.DB)
	// ListAll(true) to include revoked, or false for active?
	// Existing CLI had --all flag for revoked??
	// The existing CLI help says: "List revoked agents: attest agent list --all".
	// But `runAgentList` didn't implement logic for --all?? Wait.
	// `agentListCmd` didn't define --all flag in `init` in `cmd/agent.go`?
	// Ah, I see `agentListCmd` Example says `--all` but I don't see the flag variable `agentListAll` defined in `cmd/agent.go`.
	// The original SQL was `ORDER BY created_at DESC`, without WHERE clause! So it listed ALL agents by default.
	// I will usage `store.ListAll(true)` to match previous behavior of showing all.

	agents, err := store.ListAll(true)
	if err != nil {
		return err
	}

	if agentFormat == "json" {
		output, _ := json.MarshalIndent(agents, "", "  ")
		fmt.Println(string(output))
	} else {
		fmt.Printf("%-20s %-20s %-15s %s\n", "ID", "NAME", "TYPE", "CREATED")
		fmt.Printf("%-20s %-20s %-15s %s\n", "----", "----", "----", "-------")
		for _, a := range agents {
			status := ""
			if a.IsRevoked() {
				status = " [REVOKED]"
			}
			id := a.ID
			if len(id) > 20 {
				id = id[:20]
			}
			name := a.Name
			if len(name) > 20 {
				name = name[:20]
			}
			created := a.CreatedAt.Format(time.RFC3339)
			if len(created) > 10 {
				created = created[:10]
			}
			fmt.Printf("%-20s %-20s %-15s %s%s\n", id, name, a.Type, created, status)
		}
	}

	return nil
}

func runAgentShow(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	store := identity.NewAgentStore(db.DB)
	agent, err := store.Get(id)
	if err != nil {
		return err
	}

	if agentFormat == "json" {
		output, _ := json.MarshalIndent(agent, "", "  ")
		fmt.Println(string(output))
	} else {
		fmt.Printf("Agent ID:      %s\n", agent.ID)
		fmt.Printf("Name:          %s\n", agent.Name)
		fmt.Printf("Type:          %s\n", agent.Type)
		fmt.Printf("Public Key:    %s...\n", agent.PublicKey[:40])
		fmt.Printf("Created:       %s\n", agent.CreatedAt.Format(time.RFC3339))
		fmt.Printf("Revoked:       %v\n", agent.IsRevoked())
		if agent.IsRevoked() {
			fmt.Printf("Revoked At:    %s\n", agent.RevokedAt.Format(time.RFC3339))
		}
		// Metadata handling improvements could be added here
	}

	return nil
}

func runAgentDelete(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	store := identity.NewAgentStore(db.DB)
	if err := store.Revoke(id); err != nil {
		return err
	}

	fmt.Printf("Agent %s revoked successfully.\n", id)
	return nil
}

func runAgentExport(id string) error {
	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	var name, agentType, pubKey, createdAt, metadata string
	err = db.QueryRow(
		`SELECT name, type, public_key, created_at, metadata FROM agents WHERE id = ? AND revoked_at IS NULL`,
		id,
	).Scan(&name, &agentType, &pubKey, &createdAt, &metadata)

	if err != nil {
		return fmt.Errorf("agent not found: %s", id)
	}

	export := map[string]interface{}{
		"id":         id,
		"name":       name,
		"type":       agentType,
		"publicKey":  pubKey,
		"createdAt":  createdAt,
		"metadata":   metadata,
		"exportedAt": time.Now().UTC().Format(time.RFC3339),
	}

	output, _ := json.MarshalIndent(export, "", "  ")
	fmt.Println(string(output))
	return nil
}

func runAgentImport(path string) error {
	data, err := os.ReadFile(path)
	if err != nil {
		return fmt.Errorf("failed to read file: %w", err)
	}

	var agentData map[string]interface{}
	if err := json.Unmarshal(data, &agentData); err != nil {
		return fmt.Errorf("invalid JSON: %w", err)
	}

	db, err := storage.NewDB(cfg.DBPath)
	if err != nil {
		return fmt.Errorf("failed to open database: %w", err)
	}
	defer db.Close()

	id := agentData["id"].(string)
	name := agentData["name"].(string)
	agentType := agentData["type"].(string)
	pubKey := agentData["publicKey"].(string)
	createdAt := agentData["createdAt"].(string)
	meta, _ := json.Marshal(agentData["metadata"])

	_, err = db.Exec(
		`INSERT OR REPLACE INTO agents (id, name, type, public_key, private_key_encrypted, created_at, metadata, revoked_at) VALUES (?, ?, ?, ?, ?, ?, ?, NULL)`,
		id, name, agentType, pubKey, "", createdAt, string(meta),
	)
	if err != nil {
		return fmt.Errorf("failed to import agent: %w", err)
	}

	fmt.Printf("Agent %s imported successfully.\n", id)
	return nil
}

// generateAgentID is now handled by the exported GenerateAgentID
