package cmd

import (
	"fmt"
	"os"
	"path/filepath"

	"github.com/provnai/attest/pkg/storage"
	"github.com/spf13/cobra"
	"github.com/spf13/viper"
)

type Config struct {
	DBPath    string
	DataDir   string
	BackupDir string
	Verbose   bool
	LogLevel  string
}

var (
	cfg        *Config
	dataDir    string
	configFile string
	verbose    bool
	jsonOutput bool
)

func initConfig() {
	v := setupViper()

	if configFile != "" {
		v.SetConfigFile(configFile)
	} else {
		home, _ := os.UserHomeDir()
		v.AddConfigPath(filepath.Join(home, ".attest"))
		v.AddConfigPath(".")
		v.SetConfigName("attest")
		v.SetConfigType("yaml")
	}

	if err := v.ReadInConfig(); err != nil {
		if _, ok := err.(viper.ConfigFileNotFoundError); !ok {
			fmt.Printf("Warning: failed to read config file: %v\n", err)
		}
	}

	dataDir = v.GetString("data_dir")
	if dataDir == "" {
		home, _ := os.UserHomeDir()
		dataDir = filepath.Join(home, ".attest")
	}
	dataDir = os.ExpandEnv(dataDir)

	cfg = &Config{
		DBPath:    filepath.Join(dataDir, "attest.db"),
		DataDir:   dataDir,
		BackupDir: filepath.Join(dataDir, "backups"),
		Verbose:   verbose,
	}

	// Auto-migrate database
	db, err := storage.NewDB(cfg.DBPath)
	if err == nil {
		if err := db.Migrate(); err != nil {
			fmt.Fprintf(os.Stderr, "Warning: Database migration failed: %v\n", err)
		}
		db.Close()
	}
}

func setupViper() *viper.Viper {
	v := viper.New()

	v.SetDefault("data_dir", "$HOME/.attest")
	v.SetDefault("log_level", "info")

	v.SetEnvPrefix("ATTEST")
	v.AutomaticEnv()

	v.Set("ignore_missing_config", true)

	return v
}

var rootCmd = &cobra.Command{
	Use:   "attest",
	Short: "Attest - AI Agent Testing & Validation Tool",
	Long: `Attest provides comprehensive testing, validation, and monitoring for AI agents.

Complete documentation is available at https://github.com/provnai/attest`,
}

func Execute() error {
	initConfig()
	return rootCmd.Execute()
}

func init() {
	rootCmd.PersistentFlags().StringVar(&configFile, "config", "", "config file (default is $HOME/.attest.yaml)")
	rootCmd.PersistentFlags().BoolVar(&verbose, "verbose", false, "verbose output")
	rootCmd.PersistentFlags().BoolVar(&jsonOutput, "json", false, "output as JSON")

	// initCmd is added in cmd/init.go init()
	rootCmd.AddCommand(versionCmd)
	rootCmd.AddCommand(agentCmd)
	rootCmd.AddCommand(attestCmd)
	rootCmd.AddCommand(verifyCmd)
	rootCmd.AddCommand(intentCmd)
	rootCmd.AddCommand(execCmd)
	rootCmd.AddCommand(policyCmd)
	rootCmd.AddCommand(queryCmd)
	rootCmd.AddCommand(gitCmd)
	rootCmd.AddCommand(identityCmd)
	rootCmd.AddCommand(hardwareCmd)
}
