package cmd

import (
	"fmt"

	"github.com/spf13/cobra"
)

var versionCmd = &cobra.Command{
	Use:   "version",
	Short: "Print version information",
	Long:  `Print the version number, build info, and commit hash of Attest.`,
	Run: func(cmd *cobra.Command, args []string) {
		version := "1.0.0"
		commit := "dev"
		date := "unknown"

		fmt.Printf("Attest v%s", version)
		fmt.Printf("Commit: %s", commit)
		fmt.Printf("Date: %s", date)

		if verbose {
			fmt.Printf("Data Directory: %s", dataDir)
		}
	},
}
