package attestation

import (
	"crypto/ed25519"
	"crypto/sha256"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"time"

	"github.com/provnai/attest/pkg/crypto"
	"github.com/provnai/attest/pkg/identity"
)

// ActionType represents the type of action
type ActionType string

const (
	ActionTypeCommand  ActionType = "command"
	ActionTypeFileEdit ActionType = "file_edit"
	ActionTypeAPICall  ActionType = "api_call"
	ActionTypeDatabase ActionType = "database"
	ActionTypeGit      ActionType = "git"
	ActionTypeCustom   ActionType = "custom"
)

// Attestation represents a cryptographic attestation of an agent action
type Attestation struct {
	ID           string          `json:"id"`
	AgentID      string          `json:"agentId"`
	IntentID     string          `json:"intentId,omitempty"`
	Action       ActionRecord    `json:"action"`
	Intent       IntentRef       `json:"intentRef,omitempty"`
	Timestamp    time.Time       `json:"timestamp"`
	Signature    string          `json:"signature"`
	Verification Verification    `json:"verification"`
	Metadata     AttestationMeta `json:"metadata,omitempty"`
}

// ActionRecord represents the action that was attested
type ActionRecord struct {
	Type           ActionType `json:"type"`
	Target         string     `json:"target"`
	Input          string     `json:"input,omitempty"`
	Command        string     `json:"command,omitempty"`
	Classification string     `json:"classification,omitempty"`
}

// IntentRef references an intent
type IntentRef struct {
	ID   string `json:"id"`
	Goal string `json:"goal"`
	Link string `json:"link,omitempty"`
}

// Verification contains verification metadata
type Verification struct {
	Algorithm  string `json:"algorithm"`
	KeyType    string `json:"keyType"`
	VerifiedAt string `json:"verifiedAt"`
	Valid      bool   `json:"valid"`
	ChainValid bool   `json:"chainValid"`
}

// AttestationMeta contains additional attestation metadata
type AttestationMeta struct {
	SessionID      string   `json:"sessionId,omitempty"`
	ConversationID string   `json:"conversationId,omitempty"`
	TraceID        string   `json:"traceId,omitempty"`
	ParentID       string   `json:"parentId,omitempty"`
	Tags           []string `json:"tags,omitempty"`
}

// CreateAttestation creates a new attestation
func CreateAttestation(
	agent *identity.Agent,
	keys *crypto.KeyPair,
	action ActionRecord,
	intentID string,
	meta AttestationMeta,
) (*Attestation, error) {
	// Generate attestation ID
	attestID := generateAttestationID(action)

	// Create intent reference if provided
	var intentRef IntentRef
	if intentID != "" {
		intentRef = IntentRef{
			ID:   intentID,
			Link: fmt.Sprintf("intent:%s", intentID),
		}
	}

	// Create the attestation
	attest := &Attestation{
		ID:        attestID,
		AgentID:   agent.ID,
		IntentID:  intentID,
		Action:    action,
		Intent:    intentRef,
		Timestamp: time.Now().UTC(),
		Metadata:  meta,
	}

	// Sign the attestation
	signature, err := signAttestation(attest, keys)
	if err != nil {
		return nil, fmt.Errorf("failed to sign attestation: %w", err)
	}
	attest.Signature = signature

	// Set verification info
	attest.Verification = Verification{
		Algorithm:  "Ed25519",
		KeyType:    "Ed25519",
		VerifiedAt: time.Now().UTC().Format(time.RFC3339),
		Valid:      true,
		ChainValid: true,
	}

	return attest, nil
}

// Verify verifies an attestation
func (a *Attestation) Verify(agentPubKey []byte) bool {
	// Reconstruct canonical sign data
	signData := fmt.Sprintf("%s:%s:%s:%s",
		a.AgentID,
		a.Action.Type,
		a.Action.Target,
		a.Timestamp.Format(time.RFC3339),
	)

	// Decode signature (remove "sig:" prefix)
	if len(a.Signature) < 4 || a.Signature[:4] != "sig:" {
		return false
	}
	sigHex := a.Signature[4:]

	// Decode hex signature
	signature, err := hex.DecodeString(sigHex)
	if err != nil {
		return false
	}

	// Verify using Ed25519
	return ed25519.Verify(agentPubKey, []byte(signData), signature)
}

// ToJSON returns the attestation as JSON
func (a *Attestation) ToJSON() ([]byte, error) {
	return json.MarshalIndent(a, "", "  ")
}

// FromJSON parses an attestation from JSON
func FromJSON(data []byte) (*Attestation, error) {
	var attest Attestation
	if err := json.Unmarshal(data, &attest); err != nil {
		return nil, fmt.Errorf("failed to parse attestation: %w", err)
	}
	return &attest, nil
}

// generateAttestationID generates a unique ID for an attestation
func generateAttestationID(action ActionRecord) string {
	data := fmt.Sprintf("%s:%s:%s", action.Type, action.Target, time.Now().UTC().Format(time.RFC3339))
	hash := sha256.Sum256([]byte(data))
	return fmt.Sprintf("att:%x", hash[:8])
}

// signAttestation signs an attestation with a key pair
func signAttestation(attest *Attestation, keys *crypto.KeyPair) (string, error) {
	// Create canonical representation for signing
	signData := fmt.Sprintf("%s:%s:%s:%s",
		attest.AgentID,
		attest.Action.Type,
		attest.Action.Target,
		attest.Timestamp.Format(time.RFC3339),
	)

	sig, err := keys.Sign([]byte(signData))
	if err != nil {
		return "", err
	}

	return fmt.Sprintf("sig:%x", sig), nil
}

// AttestationStore provides storage for attestations
type AttestationStore struct {
	db *sql.DB
}

// NewAttestationStore creates a new attestation store backed by the given SQLite DB.
func NewAttestationStore(db *sql.DB) *AttestationStore {
	return &AttestationStore{db: db}
}

// Migrate creates the attestations table if it doesn't exist.
func (s *AttestationStore) Migrate() error {
	_, err := s.db.Exec(`
		CREATE TABLE IF NOT EXISTS attestations (
			id            TEXT PRIMARY KEY,
			agent_id      TEXT NOT NULL,
			intent_id     TEXT,
			action_type   TEXT NOT NULL,
			action_target TEXT NOT NULL,
			action_input  TEXT,
			signature     TEXT NOT NULL,
			timestamp     DATETIME NOT NULL,
			metadata      TEXT
		);
		CREATE INDEX IF NOT EXISTS idx_att_agent  ON attestations(agent_id);
		CREATE INDEX IF NOT EXISTS idx_att_intent ON attestations(intent_id);
		CREATE INDEX IF NOT EXISTS idx_att_ts     ON attestations(timestamp);
	`)
	return err
}

// Save stores an attestation
func (s *AttestationStore) Save(attest *Attestation) error {
	metaJSON, err := json.Marshal(attest.Metadata)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	_, err = s.db.Exec(
		`INSERT INTO attestations (id, agent_id, intent_id, action_type, action_target, action_input, signature, timestamp, metadata) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`,
		attest.ID, attest.AgentID, attest.IntentID, attest.Action.Type, attest.Action.Target, attest.Action.Input,
		attest.Signature, attest.Timestamp.Format(time.RFC3339), string(metaJSON),
	)
	if err != nil {
		return fmt.Errorf("failed to save attestation: %w", err)
	}
	return nil
}

// Get retrieves an attestation by ID
func (s *AttestationStore) Get(id string) (*Attestation, error) {
	var agentID, intentID, actionType, actionTarget, actionInput, signature, timestamp, metadata string

	err := s.db.QueryRow(
		`SELECT agent_id, intent_id, action_type, action_target, action_input, signature, timestamp, metadata FROM attestations WHERE id = ?`,
		id,
	).Scan(&agentID, &intentID, &actionType, &actionTarget, &actionInput, &signature, &timestamp, &metadata)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("attestation not found: %s", id)
		}
		return nil, fmt.Errorf("failed to query attestation: %w", err)
	}

	var meta AttestationMeta
	if metadata != "" {
		if err := json.Unmarshal([]byte(metadata), &meta); err != nil {
			return nil, fmt.Errorf("failed to parse metadata: %w", err)
		}
	}

	parsedTime, _ := time.Parse(time.RFC3339, timestamp)

	attest := &Attestation{
		ID:       id,
		AgentID:  agentID,
		IntentID: intentID,
		Action: ActionRecord{
			Type:   ActionType(actionType),
			Target: actionTarget,
			Input:  actionInput,
		},
		Timestamp: parsedTime,
		Signature: signature,
		Metadata:  meta,
	}

	return attest, nil
}

// List returns attestations with optional agentID and intentID filters.
// Pass "" to skip a filter. limit=0 defaults to 100.
func (s *AttestationStore) List(agentID, intentID string, limit int) ([]*Attestation, error) {
	if limit <= 0 {
		limit = 100
	}

	query := `SELECT id, agent_id, intent_id, action_type, action_target, action_input,
		             signature, timestamp, metadata
		        FROM attestations WHERE 1=1`
	args := []interface{}{}

	if agentID != "" {
		query += " AND agent_id = ?"
		args = append(args, agentID)
	}
	if intentID != "" {
		query += " AND intent_id = ?"
		args = append(args, intentID)
	}
	query += " ORDER BY timestamp DESC LIMIT ?"
	args = append(args, limit)

	rows, err := s.db.Query(query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to list attestations: %w", err)
	}
	defer rows.Close()

	var results []*Attestation
	for rows.Next() {
		var (
			aID, agID, iID, aType, aTarget, aInput, sig, ts string
			metadataStr                                     string
		)
		if err := rows.Scan(&aID, &agID, &iID, &aType, &aTarget, &aInput, &sig, &ts, &metadataStr); err != nil {
			return nil, err
		}
		var meta AttestationMeta
		if metadataStr != "" {
			_ = json.Unmarshal([]byte(metadataStr), &meta)
		}
		parsedTime, _ := time.Parse(time.RFC3339, ts)
		results = append(results, &Attestation{
			ID:       aID,
			AgentID:  agID,
			IntentID: iID,
			Action: ActionRecord{
				Type:   ActionType(aType),
				Target: aTarget,
				Input:  aInput,
			},
			Timestamp: parsedTime,
			Signature: sig,
			Metadata:  meta,
		})
	}
	return results, rows.Err()
}

// AttestationInfo represents attestation data for display
type AttestationInfo struct {
	ID        string    `json:"id"`
	AgentID   string    `json:"agentId"`
	IntentID  string    `json:"intentId,omitempty"`
	Action    string    `json:"action"`
	Timestamp time.Time `json:"timestamp"`
	Valid     bool      `json:"valid"`
}

// ToDisplayInfo converts an Attestation to AttestationInfo
func (a *Attestation) ToDisplayInfo() *AttestationInfo {
	return &AttestationInfo{
		ID:        a.ID,
		AgentID:   a.AgentID,
		IntentID:  a.IntentID,
		Action:    fmt.Sprintf("%s on %s", a.Action.Type, a.Action.Target),
		Timestamp: a.Timestamp,
		Valid:     a.Verification.Valid,
	}
}

// PrettyPrint prints an attestation in a human-readable format
func (a *Attestation) PrettyPrint() string {
	return fmt.Sprintf(`Attestation ID: %s
Agent:         %s
Intent:        %s
Action:        %s %s
Timestamp:     %s
Signature:     %s
Verified:      %s
`,
		a.ID,
		a.AgentID,
		a.IntentID,
		a.Action.Type,
		a.Action.Target,
		a.Timestamp.Format(time.RFC3339),
		a.Signature[:16],
		fmt.Sprintf("%v", a.Verification.Valid),
	)
}
