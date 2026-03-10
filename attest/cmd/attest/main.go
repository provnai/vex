package main

import (
	"os"

	"github.com/provnai/attest/cmd"
)

func main() {
	if err := cmd.Execute(); err != nil {
		os.Exit(1)
	}
}
