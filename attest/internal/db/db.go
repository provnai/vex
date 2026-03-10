// internal/db/db.go - Database connection management

package db

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"

	_ "modernc.org/sqlite"
)

var dbPath string

// SetDBPath sets the database file path
func SetDBPath(path string) {
	dbPath = path
}

// getDefaultDBPath returns the default database location
func getDefaultDBPath() string {
	home, err := os.UserHomeDir()
	if err != nil {
		return "attest.db"
	}
	return filepath.Join(home, ".attest", "attest.db")
}

// Open opens the database connection
func Open() (*sql.DB, error) {
	path := dbPath
	if path == "" {
		path = getDefaultDBPath()
	}

	// Ensure directory exists
	dir := filepath.Dir(path)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create database directory: %w", err)
	}

	db, err := sql.Open("sqlite", path)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Initialize schema
	if err := InitSchema(db); err != nil {
		db.Close()
		return nil, err
	}

	return db, nil
}

// OpenAtPath opens database at specific path
func OpenAtPath(path string) (*sql.DB, error) {
	SetDBPath(path)
	return Open()
}
