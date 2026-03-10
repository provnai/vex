package cmd

import (
	"fmt"
	"os"

	"github.com/provnai/attest/pkg/bridge"
	"github.com/spf13/cobra"
)

var hardwareCmd = &cobra.Command{
	Use:   "hardware",
	Short: "Hardware-backed security operations",
	Long:  `Perform cryptographic operations using hardware security (TPM/Secure Enclave) via the Rust core.`,
}

var sealCmd = &cobra.Command{
	Use:   "seal [input_file] [output_file]",
	Short: "Seal data using hardware security",
	Long:  `Encrypt data such that it can only be decrypted by the same hardware.`,
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		bridge.SetStrictHardware(strictHardware)
		inputFile := args[0]
		outputFile := args[1]

		data, err := os.ReadFile(inputFile)
		if err != nil {
			fmt.Printf("Error reading input file: %v\n", err)
			os.Exit(1)
		}

		sealed, err := bridge.Seal(data)
		if err != nil {
			fmt.Printf("Error sealing data: %v\n", err)
			os.Exit(1)
		}

		if err := os.WriteFile(outputFile, sealed, 0600); err != nil {
			fmt.Printf("Error writing output file: %v\n", err)
			os.Exit(1)
		}

		fmt.Printf("Successfully sealed %s to %s\n", inputFile, outputFile)
	},
}

var unsealCmd = &cobra.Command{
	Use:   "unseal [input_file] [output_file]",
	Short: "Unseal data using hardware security",
	Long:  `Decrypt data that was previously sealed by hardware security.`,
	Args:  cobra.ExactArgs(2),
	Run: func(cmd *cobra.Command, args []string) {
		bridge.SetStrictHardware(strictHardware)
		inputFile := args[0]
		outputFile := args[1]

		data, err := os.ReadFile(inputFile)
		if err != nil {
			fmt.Printf("Error reading input file: %v\n", err)
			os.Exit(1)
		}

		unsealed, err := bridge.Unseal(data)
		if err != nil {
			fmt.Printf("Error unsealing data: %v\n", err)
			os.Exit(1)
		}

		if err := os.WriteFile(outputFile, unsealed, 0600); err != nil {
			fmt.Printf("Error writing output file: %v\n", err)
			os.Exit(1)
		}

		fmt.Printf("Successfully unsealed %s to %s\n", inputFile, outputFile)
	},
}

var strictHardware bool

func init() {
	hardwareCmd.PersistentFlags().BoolVar(&strictHardware, "strict-hardware", false, "Enforce physical hardware security (fail if TPM is missing)")
	hardwareCmd.AddCommand(sealCmd)
	hardwareCmd.AddCommand(unsealCmd)
}
