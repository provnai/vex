package db

import (
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"time"
)

type CheckpointRecord struct {
	ID            string
	IntentID      sql.NullString
	SnapshotPath  string
	CreatedAt     time.Time
	Description   sql.NullString
	Status        string
	OperationID   sql.NullString
	OperationType sql.NullString
	FileCount     int
	DBStateCount  int
	SizeBytes     int64
	Metadata      sql.NullString
}

type GuardrailLogRecord struct {
	ID           string
	Timestamp    time.Time
	Policy       string
	PolicyName   sql.NullString
	Action       string
	Command      sql.NullString
	Details      sql.NullString
	Severity     sql.NullString
	RiskLevel    sql.NullString
	CheckpointID sql.NullString
	RunID        sql.NullString
}

func SaveCheckpoint(db *sql.DB, cp *CheckpointRecord) error {
	metadataJSON := ""
	if cp.Metadata.Valid {
		metadataJSON = cp.Metadata.String
	}

	_, err := db.Exec(`
		INSERT OR REPLACE INTO checkpoints (
			id, intent_id, snapshot_path, created_at, description,
			status, operation_id, operation_type, file_count,
			db_state_count, size_bytes, metadata
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`,
		cp.ID, cp.IntentID, cp.SnapshotPath, cp.CreatedAt, cp.Description,
		cp.Status, cp.OperationID, cp.OperationType, cp.FileCount,
		cp.DBStateCount, cp.SizeBytes, metadataJSON,
	)
	if err != nil {
		return fmt.Errorf("failed to save checkpoint: %w", err)
	}
	return nil
}

func GetCheckpoint(db *sql.DB, id string) (*CheckpointRecord, error) {
	var cp CheckpointRecord
	err := db.QueryRow(`
		SELECT id, intent_id, snapshot_path, created_at, description,
		       status, operation_id, operation_type, file_count,
		       db_state_count, size_bytes, metadata
		FROM checkpoints WHERE id = ?
	`, id).Scan(
		&cp.ID, &cp.IntentID, &cp.SnapshotPath, &cp.CreatedAt, &cp.Description,
		&cp.Status, &cp.OperationID, &cp.OperationType, &cp.FileCount,
		&cp.DBStateCount, &cp.SizeBytes, &cp.Metadata,
	)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("checkpoint not found: %s", id)
		}
		return nil, fmt.Errorf("failed to get checkpoint: %w", err)
	}
	return &cp, nil
}

func ListCheckpoints(db *sql.DB, limit int) ([]*CheckpointRecord, error) {
	rows, err := db.Query(`
		SELECT id, intent_id, snapshot_path, created_at, description,
		       status, operation_id, operation_type, file_count,
		       db_state_count, size_bytes, metadata
		FROM checkpoints
		ORDER BY created_at DESC
		LIMIT ?
	`, limit)
	if err != nil {
		return nil, fmt.Errorf("failed to list checkpoints: %w", err)
	}
	defer rows.Close()

	var checkpoints []*CheckpointRecord
	for rows.Next() {
		var cp CheckpointRecord
		err := rows.Scan(
			&cp.ID, &cp.IntentID, &cp.SnapshotPath, &cp.CreatedAt, &cp.Description,
			&cp.Status, &cp.OperationID, &cp.OperationType, &cp.FileCount,
			&cp.DBStateCount, &cp.SizeBytes, &cp.Metadata,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan checkpoint: %w", err)
		}
		checkpoints = append(checkpoints, &cp)
	}

	return checkpoints, nil
}

func UpdateCheckpointStatus(db *sql.DB, id string, status string) error {
	_, err := db.Exec(`
		UPDATE checkpoints SET status = ? WHERE id = ?
	`, status, id)
	if err != nil {
		return fmt.Errorf("failed to update checkpoint status: %w", err)
	}
	return nil
}

func generateID() string {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		return fmt.Sprintf("%x", time.Now().UnixNano())
	}
	return hex.EncodeToString(b)
}

func LogGuardrailEvent(db *sql.DB, log *GuardrailLogRecord) error {
	if log.ID == "" {
		log.ID = generateID()
	}

	detailsJSON := ""
	if log.Details.Valid {
		detailsJSON = log.Details.String
	} else if log.Details.String != "" {
		detailsJSON = log.Details.String
	}

	_, err := db.Exec(`
		INSERT INTO guardrail_logs (
			id, timestamp, policy, policy_name, action,
			command, details, severity, risk_level,
			checkpoint_id, run_id
		) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
	`,
		log.ID, log.Timestamp, log.Policy, log.PolicyName, log.Action,
		log.Command, detailsJSON, log.Severity, log.RiskLevel,
		log.CheckpointID, log.RunID,
	)
	if err != nil {
		return fmt.Errorf("failed to log guardrail event: %w", err)
	}
	return nil
}

func GetGuardrailLogs(db *sql.DB, policy string, limit int) ([]*GuardrailLogRecord, error) {
	query := `
		SELECT id, timestamp, policy, policy_name, action,
		       command, details, severity, risk_level,
		       checkpoint_id, run_id
		FROM guardrail_logs
	`
	args := []interface{}{}

	if policy != "" {
		query += " WHERE policy = ?"
		args = append(args, policy)
	}

	query += " ORDER BY timestamp DESC LIMIT ?"
	args = append(args, limit)

	rows, err := db.Query(query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to get guardrail logs: %w", err)
	}
	defer rows.Close()

	var logs []*GuardrailLogRecord
	for rows.Next() {
		var log GuardrailLogRecord
		err := rows.Scan(
			&log.ID, &log.Timestamp, &log.Policy, &log.PolicyName, &log.Action,
			&log.Command, &log.Details, &log.Severity, &log.RiskLevel,
			&log.CheckpointID, &log.RunID,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan guardrail log: %w", err)
		}
		logs = append(logs, &log)
	}

	return logs, nil
}

func MarshalMetadata(metadata map[string]interface{}) sql.NullString {
	if metadata == nil {
		return sql.NullString{Valid: false}
	}
	data, err := json.Marshal(metadata)
	if err != nil {
		return sql.NullString{Valid: false}
	}
	return sql.NullString{String: string(data), Valid: true}
}

func UnmarshalMetadata(metadata sql.NullString) map[string]interface{} {
	if !metadata.Valid || metadata.String == "" {
		return make(map[string]interface{})
	}
	var result map[string]interface{}
	if err := json.Unmarshal([]byte(metadata.String), &result); err != nil {
		return make(map[string]interface{})
	}
	return result
}
