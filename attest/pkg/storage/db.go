package storage

import (
	"database/sql"
	"fmt"
	"os"
	"path/filepath"

	_ "modernc.org/sqlite"
)

// DB wraps the SQLite database connection
type DB struct {
	*sql.DB
	path string
}

// NewDB creates a new database connection
func NewDB(dbPath string) (*DB, error) {
	// Ensure directory exists
	dir := filepath.Dir(dbPath)
	if err := os.MkdirAll(dir, 0755); err != nil {
		return nil, fmt.Errorf("failed to create db directory: %w", err)
	}

	db, err := sql.Open("sqlite", dbPath)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	return &DB{DB: db, path: dbPath}, nil
}

// Migrate runs all migrations
func (db *DB) Migrate() error {
	migrations := []string{
		// Agents table
		`CREATE TABLE IF NOT EXISTS agents (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			type TEXT NOT NULL,
			public_key BLOB NOT NULL,
			private_key_encrypted BLOB NOT NULL,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			revoked_at DATETIME,
			metadata TEXT
		)`,

		// Intents table
		`CREATE TABLE IF NOT EXISTS intents (
			id TEXT PRIMARY KEY,
			goal TEXT NOT NULL,
			description TEXT,
			ticket_id TEXT,
			constraints TEXT,
			acceptance_criteria TEXT,
			status TEXT DEFAULT 'open',
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			closed_at DATETIME,
			metadata TEXT
		)`,

		// Attestations table
		`CREATE TABLE IF NOT EXISTS attestations (
			id TEXT PRIMARY KEY,
			agent_id TEXT NOT NULL,
			intent_id TEXT,
			action_type TEXT NOT NULL,
			action_target TEXT,
			action_input TEXT,
			signature BLOB NOT NULL,
			timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
			metadata TEXT,
			FOREIGN KEY (agent_id) REFERENCES agents(id),
			FOREIGN KEY (intent_id) REFERENCES intents(id)
		)`,

		// Intent-Action links
		`CREATE TABLE IF NOT EXISTS intent_links (
			intent_id TEXT NOT NULL,
			attestation_id TEXT NOT NULL,
			PRIMARY KEY (intent_id, attestation_id),
			FOREIGN KEY (intent_id) REFERENCES intents(id),
			FOREIGN KEY (attestation_id) REFERENCES attestations(id)
		)`,

		// Reversible actions
		`CREATE TABLE IF NOT EXISTS reversible_actions (
			id TEXT PRIMARY KEY,
			attestation_id TEXT NOT NULL,
			command TEXT,
			working_dir TEXT,
			backup_path TEXT NOT NULL,
			reverse_command TEXT,
			status TEXT DEFAULT 'pending',
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			rolled_back_at DATETIME,
			FOREIGN KEY (attestation_id) REFERENCES attestations(id)
		)`,

		// Policies
		`CREATE TABLE IF NOT EXISTS policies (
			id TEXT PRIMARY KEY,
			name TEXT NOT NULL,
			condition TEXT NOT NULL,
			action TEXT NOT NULL,
			severity TEXT DEFAULT 'warning',
			enabled INTEGER DEFAULT 1,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP
		)`,

		// Indices
		`CREATE INDEX IF NOT EXISTS idx_attestations_agent ON attestations(agent_id)`,
		`CREATE INDEX IF NOT EXISTS idx_attestations_intent ON attestations(intent_id)`,
		`CREATE INDEX IF NOT EXISTS idx_intents_ticket ON intents(ticket_id)`,
		`CREATE INDEX IF NOT EXISTS idx_intents_status ON intents(status)`,
	}

	for _, m := range migrations {
		if _, err := db.Exec(m); err != nil {
			return fmt.Errorf("migration failed: %w", err)
		}
	}

	return nil
}

// Close closes the database connection
func (db *DB) Close() error {
	return db.DB.Close()
}

// Path returns the database file path
func (db *DB) Path() string {
	return db.path
}
