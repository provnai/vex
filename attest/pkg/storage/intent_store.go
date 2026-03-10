package storage

import (
	"database/sql"
	"encoding/json"
	"fmt"
	"time"

	"github.com/provnai/attest/pkg/intent"
)

// IntentStore provides database operations for intents
type IntentStore struct {
	db *DB
}

// NewIntentStore creates a new intent store
func NewIntentStore(db *DB) *IntentStore {
	return &IntentStore{db: db}
}

// SaveIntent saves an intent to the database
func (s *IntentStore) SaveIntent(i *intent.Intent) error {
	constraints, _ := json.Marshal(i.Constraints)
	criteria, _ := json.Marshal(i.AcceptanceCriteria)
	metadata, _ := json.Marshal(i.Metadata)

	_, err := s.db.Exec(
		`INSERT OR REPLACE INTO intents (id, goal, description, ticket_id, constraints, acceptance_criteria, status, created_at, closed_at, metadata)
		 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		i.ID, i.Goal, i.Description, i.TicketID, string(constraints), string(criteria), string(i.Status),
		i.CreatedAt.Format(time.RFC3339), func() *time.Time {
			if i.ClosedAt != nil {
				return i.ClosedAt
			}
			return nil
		}(), string(metadata))
	return err
}

// GetIntent retrieves an intent by ID
func (s *IntentStore) GetIntent(id string) (*intent.Intent, error) {
	var goal, description, ticketID, constraints, criteria, status, createdAt, closedAt, metadata sql.NullString
	err := s.db.QueryRow(
		`SELECT goal, description, ticket_id, constraints, acceptance_criteria, status, created_at, closed_at, metadata
		 FROM intents WHERE id = ?`, id).Scan(&goal, &description, &ticketID, &constraints, &criteria, &status, &createdAt, &closedAt, &metadata)
	if err != nil {
		return nil, err
	}

	var cons []string
	var crit []string
	var meta intent.IntentMeta
	if constraints.Valid && constraints.String != "" {
		if err := json.Unmarshal([]byte(constraints.String), &cons); err != nil {
			fmt.Printf("Warning: failed to unmarshal constraints: %v\n", err)
		}
	}
	if criteria.Valid && criteria.String != "" {
		if err := json.Unmarshal([]byte(criteria.String), &crit); err != nil {
			fmt.Printf("Warning: failed to unmarshal criteria: %v\n", err)
		}
	}
	if metadata.Valid && metadata.String != "" {
		if err := json.Unmarshal([]byte(metadata.String), &meta); err != nil {
			fmt.Printf("Warning: failed to unmarshal metadata: %v\n", err)
		}
	}

	created, _ := time.Parse(time.RFC3339, createdAt.String)
	var closed *time.Time
	if closedAt.Valid && closedAt.String != "" {
		t, _ := time.Parse(time.RFC3339, closedAt.String)
		closed = &t
	}

	return &intent.Intent{
		ID:                 id,
		Goal:               goal.String,
		Description:        description.String,
		TicketID:           ticketID.String,
		Constraints:        cons,
		AcceptanceCriteria: crit,
		Status:             intent.IntentStatus(status.String),
		CreatedAt:          created,
		ClosedAt:           closed,
		Metadata:           meta,
	}, nil
}

// ListIntents returns all intents with optional status filter
func (s *IntentStore) ListIntents(statusFilter string) ([]*intent.Intent, error) {
	query := `SELECT id, goal, description, ticket_id, constraints, acceptance_criteria, status, created_at, closed_at, metadata FROM intents`
	args := []interface{}{}
	if statusFilter != "" {
		query += ` WHERE status = ?`
		args = append(args, statusFilter)
	}
	query += ` ORDER BY created_at DESC`

	rows, err := s.db.Query(query, args...)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var intents []*intent.Intent
	for rows.Next() {
		var id, goal, description, ticketID, constraints, criteria, status, createdAt, closedAt, metadata sql.NullString
		if err := rows.Scan(&id, &goal, &description, &ticketID, &constraints, &criteria, &status, &createdAt, &closedAt, &metadata); err != nil {
			return nil, err
		}

		var cons []string
		var crit []string
		var meta intent.IntentMeta
		if constraints.Valid && constraints.String != "" {
			if err := json.Unmarshal([]byte(constraints.String), &cons); err != nil {
				fmt.Printf("Warning: failed to unmarshal constraints: %v\n", err)
			}
		}
		if criteria.Valid && criteria.String != "" {
			if err := json.Unmarshal([]byte(criteria.String), &crit); err != nil {
				fmt.Printf("Warning: failed to unmarshal criteria: %v\n", err)
			}
		}
		if metadata.Valid && metadata.String != "" {
			if err := json.Unmarshal([]byte(metadata.String), &meta); err != nil {
				fmt.Printf("Warning: failed to unmarshal metadata: %v\n", err)
			}
		}

		created, _ := time.Parse(time.RFC3339, createdAt.String)
		var closed *time.Time
		if closedAt.Valid && closedAt.String != "" {
			t, _ := time.Parse(time.RFC3339, closedAt.String)
			closed = &t
		}

		intents = append(intents, &intent.Intent{
			ID:                 id.String,
			Goal:               goal.String,
			Description:        description.String,
			TicketID:           ticketID.String,
			Constraints:        cons,
			AcceptanceCriteria: crit,
			Status:             intent.IntentStatus(status.String),
			CreatedAt:          created,
			ClosedAt:           closed,
			Metadata:           meta,
		})
	}
	return intents, nil
}

// GetIntentByTicket retrieves an intent by ticket ID
func (s *IntentStore) GetIntentByTicket(ticketID string) (*intent.Intent, error) {
	rows, err := s.db.Query(
		`SELECT id FROM intents WHERE ticket_id = ?`, ticketID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var id string
	if rows.Next() {
		if err := rows.Scan(&id); err != nil {
			return nil, fmt.Errorf("failed to scan intent id: %w", err)
		}
	}
	if id == "" {
		return nil, fmt.Errorf("intent not found for ticket: %s", ticketID)
	}
	return s.GetIntent(id)
}

// Close closes the store
func (s *IntentStore) Close() error {
	return nil
}
