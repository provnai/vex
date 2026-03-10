package cmd

import (
	"encoding/base64"
	"fmt"
	"time"

	"github.com/provnai/attest/pkg/attestation"
	"github.com/provnai/attest/pkg/identity"
	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
)

var verifyCmd = &cobra.Command{
	Use:   "verify",
	Short: "Verify attestations",
	Long:  `Verify the authenticity and integrity of attestations.`,
}

func init() {
	verifyCmd.AddCommand(verifyCheckCmd)
}

var verifyCheckCmd = &cobra.Command{
	Use:   "check [id]",
	Short: "Verify an attestation by ID",
	Long:  `Verify that an attestation is valid, authentic, and has not been tampered with.`,
	Args:  cobra.ExactArgs(1),
	RunE: func(cmd *cobra.Command, args []string) error {
		attestationID := args[0]

		db, err := storage.NewDB(cfg.DBPath)
		if err != nil {
			return fmt.Errorf("failed to open database: %w", err)
		}
		defer db.Close()

		// 1. Get Attestation
		attestStore := attestation.NewAttestationStore(db.DB)
		attest, err := attestStore.Get(attestationID)
		if err != nil {
			return err
		}

		// 2. Get Agent for Public Key
		agentStore := identity.NewAgentStore(db.DB)
		agent, err := agentStore.Get(attest.AgentID)
		if err != nil {
			return fmt.Errorf("failed to get agent %s: %w", attest.AgentID, err)
		}

		// 3. Verify
		// Decode the public key from base64
		pubKey, err := base64.StdEncoding.DecodeString(agent.PublicKey)
		if err != nil {
			return fmt.Errorf("invalid agent public key: %w", err)
		}

		valid := attest.Verify(pubKey)

		// 4. Output
		fmt.Printf("Attestation: %s\n", attest.ID)
		fmt.Printf("Agent:       %s (%s)\n", agent.ID, agent.Name)
		fmt.Printf("Action:      %s %s\n", attest.Action.Type, attest.Action.Target)
		fmt.Printf("Timestamp:   %s\n", attest.Timestamp.Format(time.RFC3339))
		fmt.Println()

		if valid {
			fmt.Printf("✅ VERIFIED VALID\n")
			fmt.Printf("Signature is cryptographically valid and linked to agent %s.\n", agent.ID)
		} else {
			fmt.Printf("❌ VERIFICATION FAILED\n")
			fmt.Printf("Signature does not match content or agent key.\n")
			return fmt.Errorf("verification failed")
		}

		return nil
	},
}
