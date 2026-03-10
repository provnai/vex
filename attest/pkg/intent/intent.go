package intent

import (
	"context"
	"crypto/sha256"
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	_ "github.com/mattn/go-sqlite3"
)

// IntentStatus represents the status of an intent
type IntentStatus string

const (
	IntentStatusOpen     IntentStatus = "open"
	IntentStatusProgress IntentStatus = "in_progress"
	IntentStatusComplete IntentStatus = "completed"
	IntentStatusFailed   IntentStatus = "failed"
	IntentStatusCanceled IntentStatus = "canceled"
)

// Intent represents an agent intent/goal
type Intent struct {
	ID                 string       `json:"id"`
	Goal               string       `json:"goal"`
	Description        string       `json:"description,omitempty"`
	TicketID           string       `json:"ticketId,omitempty"`
	Constraints        []string     `json:"constraints,omitempty"`
	AcceptanceCriteria []string     `json:"acceptanceCriteria,omitempty"`
	Status             IntentStatus `json:"status"`
	CreatedAt          time.Time    `json:"createdAt"`
	ClosedAt           *time.Time   `json:"closedAt,omitempty"`
	Metadata           IntentMeta   `json:"metadata,omitempty"`
}

// IntentMeta contains additional metadata about an intent
type IntentMeta struct {
	Priority   string            `json:"priority,omitempty"`
	Assignee   string            `json:"assignee,omitempty"`
	Epic       string            `json:"epic,omitempty"`
	Labels     []string          `json:"labels,omitempty"`
	CustomData map[string]string `json:"customData,omitempty"`
}

// CreateIntent creates a new intent
func CreateIntent(goal string, description, ticketID string, constraints, criteria []string) *Intent {
	id := generateIntentID(goal)

	return &Intent{
		ID:                 id,
		Goal:               goal,
		Description:        description,
		TicketID:           ticketID,
		Constraints:        constraints,
		AcceptanceCriteria: criteria,
		Status:             IntentStatusOpen,
		CreatedAt:          time.Now().UTC(),
	}
}

// Close marks an intent as completed
func (i *Intent) Close(success bool) {
	now := time.Now().UTC()
	i.ClosedAt = &now
	if success {
		i.Status = IntentStatusComplete
	} else {
		i.Status = IntentStatusFailed
	}
}

// Progress marks intent as in progress
func (i *Intent) Progress() {
	i.Status = IntentStatusProgress
}

// Cancel cancels the intent
func (i *Intent) Cancel() {
	now := time.Now().UTC()
	i.ClosedAt = &now
	i.Status = IntentStatusCanceled
}

// ToJSON returns the intent as JSON
func (i *Intent) ToJSON() ([]byte, error) {
	return json.MarshalIndent(i, "", "  ")
}

// FromJSON parses an intent from JSON
func FromJSON(data []byte) (*Intent, error) {
	var intent Intent
	if err := json.Unmarshal(data, &intent); err != nil {
		return nil, fmt.Errorf("failed to parse intent: %w", err)
	}
	return &intent, nil
}

// generateIntentID generates a unique ID for an intent
func generateIntentID(goal string) string {
	data := fmt.Sprintf("intent:%s:%s", goal, time.Now().UTC().Format(time.RFC3339Nano))
	hash := sha256.Sum256([]byte(data))
	return fmt.Sprintf("int:%x", hash[:8])
}

// IntentStore provides SQLite-backed storage for intents.
// Task 5: All methods are real implementations backed by SQLite.
type IntentStore struct {
	db *sql.DB
}

// NewIntentStore creates a new intent store backed by the given SQLite DB.
// The caller is responsible for running migrations (see Migrate).
func NewIntentStore(db *sql.DB) *IntentStore {
	return &IntentStore{db: db}
}

// Migrate creates the intents and intent_attestations tables if they don't exist.
func (s *IntentStore) Migrate(ctx context.Context) error {
	_, err := s.db.ExecContext(ctx, `
		CREATE TABLE IF NOT EXISTS intents (
			id           TEXT PRIMARY KEY,
			goal         TEXT NOT NULL,
			description  TEXT,
			ticket_id    TEXT,
			status       TEXT NOT NULL DEFAULT 'open',
			constraints  TEXT,   -- JSON array
			criteria     TEXT,   -- JSON array
			metadata     TEXT,   -- JSON object
			created_at   DATETIME NOT NULL,
			closed_at    DATETIME
		);
		CREATE INDEX IF NOT EXISTS idx_intents_status ON intents(status);
		CREATE INDEX IF NOT EXISTS idx_intents_ticket ON intents(ticket_id);

		CREATE TABLE IF NOT EXISTS intent_attestations (
			intent_id      TEXT NOT NULL,
			attestation_id TEXT NOT NULL,
			linked_at      DATETIME NOT NULL DEFAULT CURRENT_TIMESTAMP,
			PRIMARY KEY (intent_id, attestation_id),
			FOREIGN KEY (intent_id) REFERENCES intents(id)
		);
	`)
	return err
}

// Save stores an intent (upsert by ID).
func (s *IntentStore) Save(intent *Intent) error {
	constraints, _ := json.Marshal(intent.Constraints)
	criteria, _ := json.Marshal(intent.AcceptanceCriteria)
	meta, _ := json.Marshal(intent.Metadata)

	var closedAt *string
	if intent.ClosedAt != nil {
		t := intent.ClosedAt.UTC().Format(time.RFC3339)
		closedAt = &t
	}

	_, err := s.db.Exec(`
		INSERT INTO intents
			(id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at)
		VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
		ON CONFLICT(id) DO UPDATE SET
			goal        = excluded.goal,
			description = excluded.description,
			ticket_id   = excluded.ticket_id,
			status      = excluded.status,
			constraints = excluded.constraints,
			criteria    = excluded.criteria,
			metadata    = excluded.metadata,
			closed_at   = excluded.closed_at
	`,
		intent.ID,
		intent.Goal,
		intent.Description,
		intent.TicketID,
		string(intent.Status),
		string(constraints),
		string(criteria),
		string(meta),
		intent.CreatedAt.UTC().Format(time.RFC3339),
		closedAt,
	)
	return err
}

// Get retrieves an intent by ID.
func (s *IntentStore) Get(id string) (*Intent, error) {
	row := s.db.QueryRow(`
		SELECT id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at
		FROM intents WHERE id = ?
	`, id)
	return scanIntent(row)
}

// List returns intents with optional status filtering. Pass "" to return all.
func (s *IntentStore) List(status IntentStatus, limit int) ([]*Intent, error) {
	var rows *sql.Rows
	var err error

	if status == "" {
		rows, err = s.db.Query(`
			SELECT id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at
			FROM intents ORDER BY created_at DESC LIMIT ?
		`, limit)
	} else {
		rows, err = s.db.Query(`
			SELECT id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at
			FROM intents WHERE status = ? ORDER BY created_at DESC LIMIT ?
		`, string(status), limit)
	}
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanIntents(rows)
}

// FindByTicket finds an intent by ticket ID.
func (s *IntentStore) FindByTicket(ticketID string) (*Intent, error) {
	row := s.db.QueryRow(`
		SELECT id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at
		FROM intents WHERE ticket_id = ? ORDER BY created_at DESC LIMIT 1
	`, ticketID)
	return scanIntent(row)
}

// FindByGoal searches intents by goal text (case-insensitive LIKE).
func (s *IntentStore) FindByGoal(search string, limit int) ([]*Intent, error) {
	rows, err := s.db.Query(`
		SELECT id, goal, description, ticket_id, status, constraints, criteria, metadata, created_at, closed_at
		FROM intents WHERE goal LIKE ? ORDER BY created_at DESC LIMIT ?
	`, "%"+search+"%", limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()
	return scanIntents(rows)
}

// LinkAttestation links an attestation to an intent.
func (s *IntentStore) LinkAttestation(intentID, attestationID string) error {
	_, err := s.db.Exec(`
		INSERT OR IGNORE INTO intent_attestations (intent_id, attestation_id, linked_at)
		VALUES (?, ?, ?)
	`, intentID, attestationID, time.Now().UTC().Format(time.RFC3339))
	return err
}

// GetAttestations returns all attestation IDs linked to an intent.
func (s *IntentStore) GetAttestations(intentID string) ([]string, error) {
	rows, err := s.db.Query(`
		SELECT attestation_id FROM intent_attestations WHERE intent_id = ? ORDER BY linked_at
	`, intentID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var ids []string
	for rows.Next() {
		var id string
		if err := rows.Scan(&id); err != nil {
			return nil, err
		}
		ids = append(ids, id)
	}
	return ids, rows.Err()
}

// scanIntent scans a single *sql.Row into an Intent.
func scanIntent(row *sql.Row) (*Intent, error) {
	var (
		i              Intent
		constraintsRaw []byte
		criteriaRaw    []byte
		metaRaw        []byte
		createdAt      string
		closedAt       sql.NullString
	)
	err := row.Scan(
		&i.ID, &i.Goal, &i.Description, &i.TicketID, &i.Status,
		&constraintsRaw, &criteriaRaw, &metaRaw, &createdAt, &closedAt,
	)
	if err == sql.ErrNoRows {
		return nil, nil
	}
	if err != nil {
		return nil, err
	}
	return hydrateIntent(&i, constraintsRaw, criteriaRaw, metaRaw, createdAt, closedAt)
}

// scanIntents scans multiple *sql.Rows into a slice of Intents.
func scanIntents(rows *sql.Rows) ([]*Intent, error) {
	var results []*Intent
	for rows.Next() {
		var (
			i              Intent
			constraintsRaw []byte
			criteriaRaw    []byte
			metaRaw        []byte
			createdAt      string
			closedAt       sql.NullString
		)
		if err := rows.Scan(
			&i.ID, &i.Goal, &i.Description, &i.TicketID, &i.Status,
			&constraintsRaw, &criteriaRaw, &metaRaw, &createdAt, &closedAt,
		); err != nil {
			return nil, err
		}
		intent, err := hydrateIntent(&i, constraintsRaw, criteriaRaw, metaRaw, createdAt, closedAt)
		if err != nil {
			return nil, err
		}
		results = append(results, intent)
	}
	return results, rows.Err()
}

// hydrateIntent deserializes JSON fields and parses times.
func hydrateIntent(
	i *Intent,
	constraintsRaw, criteriaRaw, metaRaw []byte,
	createdAt string,
	closedAt sql.NullString,
) (*Intent, error) {
	if len(constraintsRaw) > 0 {
		_ = json.Unmarshal(constraintsRaw, &i.Constraints)
	}
	if len(criteriaRaw) > 0 {
		_ = json.Unmarshal(criteriaRaw, &i.AcceptanceCriteria)
	}
	if len(metaRaw) > 0 {
		_ = json.Unmarshal(metaRaw, &i.Metadata)
	}

	t, err := time.Parse(time.RFC3339, createdAt)
	if err != nil {
		return nil, fmt.Errorf("intent created_at parse: %w", err)
	}
	i.CreatedAt = t

	if closedAt.Valid && closedAt.String != "" {
		ct, err := time.Parse(time.RFC3339, closedAt.String)
		if err != nil {
			return nil, fmt.Errorf("intent closed_at parse: %w", err)
		}
		i.ClosedAt = &ct
	}

	return i, nil
}

// IntentInfo represents intent data for display
type IntentInfo struct {
	ID       string       `json:"id"`
	Goal     string       `json:"goal"`
	TicketID string       `json:"ticketId,omitempty"`
	Status   IntentStatus `json:"status"`
	Actions  int          `json:"actionCount"`
	Created  time.Time    `json:"createdAt"`
}

// ToDisplayInfo converts an Intent to IntentInfo for display
func (i *Intent) ToDisplayInfo(actionCount int) *IntentInfo {
	return &IntentInfo{
		ID:       i.ID,
		Goal:     i.Goal,
		TicketID: i.TicketID,
		Status:   i.Status,
		Actions:  actionCount,
		Created:  i.CreatedAt,
	}
}

// PrettyPrint prints an intent in a human-readable format
func (i *Intent) PrettyPrint() string {
	return fmt.Sprintf(`Intent ID:      %s
Goal:           %s
Description:    %s
Ticket:         %s
Status:         %s
Created:        %s
Constraints:    %v
Criteria:       %v
`,
		i.ID,
		i.Goal,
		i.Description,
		i.TicketID,
		i.Status,
		i.CreatedAt.Format(time.RFC3339),
		i.Constraints,
		i.AcceptanceCriteria,
	)
}

// IntentGraph represents a graph of intents and their actions
type IntentGraph struct {
	Root  *Intent      `json:"root"`
	Links []IntentLink `json:"links"`
}

// IntentLink represents a link between intent and attestation
type IntentLink struct {
	IntentID      string `json:"intentId"`
	AttestationID string `json:"attestationId"`
	ActionType    string `json:"actionType"`
	Timestamp     string `json:"timestamp"`
}
