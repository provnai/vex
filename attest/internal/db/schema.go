// internal/db/schema.go - Database schema for cost tracking

package db

import (
	"database/sql"
	"fmt"
)

// SchemaVersion tracks database schema version
const SchemaVersion = 5

// InitSchema creates all database tables including cost tracking and watch sessions
func InitSchema(db *sql.DB) error {
	// Create cost tracking table
	_, err := db.Exec(`
		CREATE TABLE IF NOT EXISTS cost_tracking (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			date DATE NOT NULL,
			model TEXT NOT NULL,
			provider TEXT NOT NULL,
			input_tokens INTEGER NOT NULL DEFAULT 0,
			output_tokens INTEGER NOT NULL DEFAULT 0,
			input_cost REAL NOT NULL DEFAULT 0,
			output_cost REAL NOT NULL DEFAULT 0,
			total_cost REAL NOT NULL DEFAULT 0,
			run_id TEXT,
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create cost_tracking table: %w", err)
	}

	// Create watch sessions table
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS watch_sessions (
			id TEXT PRIMARY KEY,
			intent_id TEXT,
			command TEXT NOT NULL,
			args TEXT,
			working_dir TEXT,
			start_time TIMESTAMP NOT NULL,
			end_time TIMESTAMP,
			process_id INTEGER,
			exit_code INTEGER,
			total_cost REAL DEFAULT 0,
			total_duration REAL,
			status TEXT DEFAULT 'running',
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create watch_sessions table: %w", err)
	}

	// Create captured actions table
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS captured_actions (
			id TEXT PRIMARY KEY,
			session_id TEXT NOT NULL,
			type TEXT NOT NULL,
			timestamp TIMESTAMP NOT NULL,
			duration REAL,
			process_id INTEGER,
			thread_id INTEGER,
			data TEXT,
			cost REAL DEFAULT 0,
			success INTEGER DEFAULT 1,
			error TEXT,
			FOREIGN KEY (session_id) REFERENCES watch_sessions(id)
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create captured_actions table: %w", err)
	}

	// Create budget config table
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS budget_config (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			daily_limit REAL NOT NULL DEFAULT 10.0,
			weekly_limit REAL NOT NULL DEFAULT 50.0,
			monthly_limit REAL NOT NULL DEFAULT 200.0,
			hard_stop BOOLEAN NOT NULL DEFAULT 1,
			warn_threshold REAL NOT NULL DEFAULT 0.8,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create budget_config table: %w", err)
	}

	// Insert default budget config if not exists
	_, err = db.Exec(`
		INSERT OR IGNORE INTO budget_config (id, daily_limit, weekly_limit, monthly_limit)
		VALUES (1, 10.0, 50.0, 200.0)
	`)
	if err != nil {
		return fmt.Errorf("failed to insert default budget config: %w", err)
	}

	// Create integrations table for tracking configured frameworks
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS integrations (
			id INTEGER PRIMARY KEY AUTOINCREMENT,
			framework TEXT NOT NULL UNIQUE,
			config_path TEXT NOT NULL,
			version TEXT NOT NULL,
			installed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
			settings TEXT
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create integrations table: %w", err)
	}

	// Create checkpoints table for guardrails
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS checkpoints (
			id TEXT PRIMARY KEY,
			intent_id TEXT,
			snapshot_path TEXT NOT NULL,
			created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
			description TEXT,
			status TEXT DEFAULT 'active',
			operation_id TEXT,
			operation_type TEXT,
			file_count INTEGER DEFAULT 0,
			db_state_count INTEGER DEFAULT 0,
			size_bytes INTEGER DEFAULT 0,
			metadata TEXT
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create checkpoints table: %w", err)
	}

	// Create guardrail_logs table for policy enforcement tracking
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS guardrail_logs (
			id TEXT PRIMARY KEY,
			timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
			policy TEXT NOT NULL,
			policy_name TEXT,
			action TEXT NOT NULL,
			command TEXT,
			details TEXT,
			severity TEXT,
			risk_level TEXT,
			checkpoint_id TEXT,
			run_id TEXT
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create guardrail_logs table: %w", err)
	}

	// Create watch sessions table for auto-instrumentation
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS watch_sessions (
			id TEXT PRIMARY KEY,
			intent_id TEXT,
			command TEXT NOT NULL,
			args TEXT,
			working_dir TEXT,
			start_time TIMESTAMP NOT NULL,
			end_time TIMESTAMP,
			process_id INTEGER,
			exit_code INTEGER,
			total_cost REAL DEFAULT 0,
			total_duration REAL,
			status TEXT DEFAULT 'running',
			created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create watch_sessions table: %w", err)
	}

	// Create captured_actions table for all instrumented actions
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS captured_actions (
			id TEXT PRIMARY KEY,
			session_id TEXT NOT NULL,
			type TEXT NOT NULL,
			timestamp TIMESTAMP NOT NULL,
			duration REAL,
			process_id INTEGER,
			thread_id INTEGER,
			data TEXT,
			cost REAL DEFAULT 0,
			success INTEGER DEFAULT 1,
			error TEXT,
			FOREIGN KEY (session_id) REFERENCES watch_sessions(id)
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create captured_actions table: %w", err)
	}

	// Create schema version table
	_, err = db.Exec(`
		CREATE TABLE IF NOT EXISTS schema_version (
			version INTEGER PRIMARY KEY,
			applied_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
		)
	`)
	if err != nil {
		return fmt.Errorf("failed to create schema_version table: %w", err)
	}

	// Insert/update schema version
	_, err = db.Exec(`
		INSERT OR REPLACE INTO schema_version (version) VALUES (?)
	`, SchemaVersion)
	if err != nil {
		return fmt.Errorf("failed to update schema version: %w", err)
	}

	return nil
}

// MigrateFromV1 migrates from schema version 1 to 2 (adds cost tracking)
func MigrateFromV1(db *sql.DB) error {
	// Check if cost_tracking exists
	var exists bool
	err := db.QueryRow(`
		SELECT EXISTS (
			SELECT 1 FROM sqlite_master
			WHERE type='table' AND name='cost_tracking'
		)
	`).Scan(&exists)

	if err != nil {
		return fmt.Errorf("failed to check if cost_tracking exists: %w", err)
	}

	if !exists {
		return InitSchema(db)
	}

	return nil
}

// MigrateFromV2 migrates from schema version 2 to 3 (adds integrations table)
func MigrateFromV2(db *sql.DB) error {
	var exists bool
	err := db.QueryRow(`
		SELECT EXISTS (
			SELECT 1 FROM sqlite_master
			WHERE type='table' AND name='integrations'
		)
	`).Scan(&exists)

	if err != nil {
		return fmt.Errorf("failed to check if integrations exists: %w", err)
	}

	if !exists {
		_, err = db.Exec(`
			CREATE TABLE IF NOT EXISTS integrations (
				id INTEGER PRIMARY KEY AUTOINCREMENT,
				framework TEXT NOT NULL UNIQUE,
				config_path TEXT NOT NULL,
				version TEXT NOT NULL,
				installed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
				updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
				settings TEXT
			)
		`)
		if err != nil {
			return fmt.Errorf("failed to create integrations table: %w", err)
		}

		_, err = db.Exec(`
			INSERT OR REPLACE INTO schema_version (version) VALUES (?)
		`, SchemaVersion)
		if err != nil {
			return fmt.Errorf("failed to update schema version: %w", err)
		}
	}

	return nil
}

// MigrateFromV3 migrates from schema version 3 to 4 (adds guardrails tables)
func MigrateFromV3(db *sql.DB) error {
	var checkpointsExists bool
	err := db.QueryRow(`
		SELECT EXISTS (
			SELECT 1 FROM sqlite_master
			WHERE type='table' AND name='checkpoints'
		)
	`).Scan(&checkpointsExists)

	if err != nil {
		return fmt.Errorf("failed to check if checkpoints exists: %w", err)
	}

	if !checkpointsExists {
		_, err = db.Exec(`
			CREATE TABLE IF NOT EXISTS checkpoints (
				id TEXT PRIMARY KEY,
				intent_id TEXT,
				snapshot_path TEXT NOT NULL,
				created_at DATETIME DEFAULT CURRENT_TIMESTAMP,
				description TEXT,
				status TEXT DEFAULT 'active',
				operation_id TEXT,
				operation_type TEXT,
				file_count INTEGER DEFAULT 0,
				db_state_count INTEGER DEFAULT 0,
				size_bytes INTEGER DEFAULT 0,
				metadata TEXT
			)
		`)
		if err != nil {
			return fmt.Errorf("failed to create checkpoints table: %w", err)
		}

		_, err = db.Exec(`
			CREATE TABLE IF NOT EXISTS guardrail_logs (
				id TEXT PRIMARY KEY,
				timestamp DATETIME DEFAULT CURRENT_TIMESTAMP,
				policy TEXT NOT NULL,
				policy_name TEXT,
				action TEXT NOT NULL,
				command TEXT,
				details TEXT,
				severity TEXT,
				risk_level TEXT,
				checkpoint_id TEXT,
				run_id TEXT
			)
		`)
		if err != nil {
			return fmt.Errorf("failed to create guardrail_logs table: %w", err)
		}

		_, err = db.Exec(`
			INSERT OR REPLACE INTO schema_version (version) VALUES (?)
		`, SchemaVersion)
		if err != nil {
			return fmt.Errorf("failed to update schema version: %w", err)
		}
	}

	return nil
}

// MigrateFromV4 migrates from schema version 4 to 5 (adds watch sessions)
func MigrateFromV4(db *sql.DB) error {
	var sessionsExists bool
	err := db.QueryRow(`
		SELECT EXISTS (
			SELECT 1 FROM sqlite_master
			WHERE type='table' AND name='watch_sessions'
		)
	`).Scan(&sessionsExists)

	if err != nil {
		return fmt.Errorf("failed to check if watch_sessions exists: %w", err)
	}

	if !sessionsExists {
		_, err = db.Exec(`
			CREATE TABLE IF NOT EXISTS watch_sessions (
				id TEXT PRIMARY KEY,
				intent_id TEXT,
				command TEXT NOT NULL,
				args TEXT,
				working_dir TEXT,
				start_time TIMESTAMP NOT NULL,
				end_time TIMESTAMP,
				process_id INTEGER,
				exit_code INTEGER,
				total_cost REAL DEFAULT 0,
				total_duration REAL,
				status TEXT DEFAULT 'running',
				created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
			)
		`)
		if err != nil {
			return fmt.Errorf("failed to create watch_sessions table: %w", err)
		}

		_, err = db.Exec(`
			CREATE TABLE IF NOT EXISTS captured_actions (
				id TEXT PRIMARY KEY,
				session_id TEXT NOT NULL,
				type TEXT NOT NULL,
				timestamp TIMESTAMP NOT NULL,
				duration REAL,
				process_id INTEGER,
				thread_id INTEGER,
				data TEXT,
				cost REAL DEFAULT 0,
				success INTEGER DEFAULT 1,
				error TEXT,
				FOREIGN KEY (session_id) REFERENCES watch_sessions(id)
			)
		`)
		if err != nil {
			return fmt.Errorf("failed to create captured_actions table: %w", err)
		}

		_, err = db.Exec(`
			INSERT OR REPLACE INTO schema_version (version) VALUES (?)
		`, SchemaVersion)
		if err != nil {
			return fmt.Errorf("failed to update schema version: %w", err)
		}
	}

	return nil
}
