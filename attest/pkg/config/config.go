package config

import (
	"os"
	"path/filepath"
)

// Config holds all configuration for attest
type Config struct {
	DataDir   string `mapstructure:"data_dir"`
	LogLevel  string `mapstructure:"log_level"`
	DBPath    string `mapstructure:"db_path"`
	PolicyDir string `mapstructure:"policy_dir"`
	BackupDir string `mapstructure:"backup_dir"`
	Verbose   bool   `mapstructure:"verbose"`
}

// DefaultConfig returns the default configuration
func DefaultConfig() *Config {
	home, _ := os.UserHomeDir()
	dataDir := filepath.Join(home, ".attest")

	return &Config{
		DataDir:   dataDir,
		LogLevel:  "info",
		DBPath:    filepath.Join(dataDir, "attest.db"),
		PolicyDir: filepath.Join(dataDir, "policies"),
		BackupDir: filepath.Join(dataDir, "backups"),
		Verbose:   false,
	}
}

// EnsureDirs creates necessary directories
func (c *Config) EnsureDirs() error {
	dirs := []string{c.DataDir, c.PolicyDir, c.BackupDir}
	for _, dir := range dirs {
		if err := os.MkdirAll(dir, 0755); err != nil {
			return err
		}
	}
	return nil
}
