package identity

import (
	"database/sql"
	"encoding/base64"
	"encoding/json"
	"fmt"
	"time"

	"github.com/provnai/attest/pkg/crypto"
)

// AgentType represents the type of AI agent
type AgentType string

const (
	AgentTypeGeneric   AgentType = "generic"
	AgentTypeLangChain AgentType = "langchain"
	AgentTypeAutoGen   AgentType = "autogen"
	AgentTypeCrewAI    AgentType = "crewai"
	AgentTypeCustom    AgentType = "custom"
)

// Agent represents an agent identity document (AID)
type Agent struct {
	ID                  string     `json:"id"`
	Name                string     `json:"name"`
	Type                AgentType  `json:"type"`
	PublicKey           string     `json:"publicKey"`
	CreatedAt           time.Time  `json:"createdAt"`
	RevokedAt           *time.Time `json:"revokedAt,omitempty"`
	Metadata            AgentMeta  `json:"metadata,omitempty"`
	PrivateKeyEncrypted string     `json:"-"` // Internal use only
}

// AgentMeta contains additional metadata about an agent
type AgentMeta struct {
	Version    string            `json:"version,omitempty"`
	Framework  string            `json:"framework,omitempty"`
	Model      string            `json:"model,omitempty"`
	Owner      string            `json:"owner,omitempty"`
	Tags       []string          `json:"tags,omitempty"`
	CustomData map[string]string `json:"customData,omitempty"`
}

// CreateAgent creates a new agent with cryptographic identity
func CreateAgent(name string, agentType AgentType, keys *crypto.KeyPair, meta AgentMeta) (*Agent, error) {
	agentID := keys.AgentID()

	agent := &Agent{
		ID:        agentID,
		Name:      name,
		Type:      agentType,
		PublicKey: keys.PublicKeyBase64(),
		CreatedAt: time.Now().UTC(),
		Metadata:  meta,
	}

	return agent, nil
}

// IsRevoked returns true if the agent has been revoked
func (a *Agent) IsRevoked() bool {
	return a.RevokedAt != nil
}

// Revoke marks the agent as revoked
func (a *Agent) Revoke() {
	now := time.Now().UTC()
	a.RevokedAt = &now
}

// ToJSON returns the agent as JSON
func (a *Agent) ToJSON() ([]byte, error) {
	return json.MarshalIndent(a, "", "  ")
}

// FromJSON parses an agent from JSON
func FromJSON(data []byte) (*Agent, error) {
	var agent Agent
	if err := json.Unmarshal(data, &agent); err != nil {
		return nil, fmt.Errorf("failed to parse agent: %w", err)
	}
	return &agent, nil
}

// AgentStore provides storage for agents
type AgentStore struct {
	db *sql.DB
}

// NewAgentStore creates a new agent store
func NewAgentStore(db *sql.DB) *AgentStore {
	return &AgentStore{
		db: db,
	}
}

// Save stores an agent in the database
func (s *AgentStore) Save(agent *Agent, keys *crypto.KeyPair) error {
	// Deprecated: use SaveWithEncryptedKey instead
	return fmt.Errorf("use SaveWithEncryptedKey instead")
}

// SaveWithEncryptedKey stores an agent with an already encrypted private key
func (s *AgentStore) SaveWithEncryptedKey(agent *Agent, encryptedPrivateKey string) error {
	metaJSON, err := json.Marshal(agent.Metadata)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	_, err = s.db.Exec(
		`INSERT INTO agents (id, name, type, public_key, private_key_encrypted, created_at, metadata) VALUES (?, ?, ?, ?, ?, ?, ?)`,
		agent.ID, agent.Name, string(agent.Type), agent.PublicKey, encryptedPrivateKey,
		agent.CreatedAt.Format(time.RFC3339), string(metaJSON),
	)
	if err != nil {
		return fmt.Errorf("failed to save agent: %w", err)
	}
	return nil
}

// Get retrieves an agent by ID
func (s *AgentStore) Get(id string) (*Agent, error) {
	var name, agentType, pubKey, privKeyEnc, createdAt, metadata string
	var revokedAt sql.NullString

	err := s.db.QueryRow(
		`SELECT name, type, public_key, private_key_encrypted, created_at, revoked_at, metadata FROM agents WHERE id = ?`,
		id,
	).Scan(&name, &agentType, &pubKey, &privKeyEnc, &createdAt, &revokedAt, &metadata)

	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("agent not found: %s", id)
		}
		return nil, fmt.Errorf("failed to query agent: %w", err)
	}

	var meta AgentMeta
	if metadata != "" {
		if err := json.Unmarshal([]byte(metadata), &meta); err != nil {
			return nil, fmt.Errorf("failed to parse metadata: %w", err)
		}
	}

	parsedTime, _ := time.Parse(time.RFC3339, createdAt)

	agent := &Agent{
		ID:                  id,
		Name:                name,
		Type:                AgentType(agentType),
		PublicKey:           pubKey,
		PrivateKeyEncrypted: privKeyEnc,
		CreatedAt:           parsedTime,
		Metadata:            meta,
	}

	if revokedAt.Valid && revokedAt.String != "" {
		revokedTime, _ := time.Parse(time.RFC3339, revokedAt.String)
		agent.RevokedAt = &revokedTime
	}

	return agent, nil
}

// List returns all non-revoked agents
func (s *AgentStore) List() ([]*Agent, error) {
	return s.ListAll(false)
}

// ListAll returns agents, optionally including revoked ones
func (s *AgentStore) ListAll(includeRevoked bool) ([]*Agent, error) {
	query := `SELECT id, name, type, public_key, created_at, revoked_at, metadata FROM agents`
	if !includeRevoked {
		query += ` WHERE revoked_at IS NULL`
	}
	query += ` ORDER BY created_at DESC`

	rows, err := s.db.Query(query)
	if err != nil {
		return nil, fmt.Errorf("failed to list agents: %w", err)
	}
	defer rows.Close()

	var agents []*Agent
	for rows.Next() {
		var id, name, agentType, pubKey, createdAt, metadata string
		var revokedAt sql.NullString

		if err := rows.Scan(&id, &name, &agentType, &pubKey, &createdAt, &revokedAt, &metadata); err != nil {
			return nil, err
		}

		var meta AgentMeta
		if err := json.Unmarshal([]byte(metadata), &meta); err != nil {
			return nil, fmt.Errorf("failed to unmarshal metadata: %w", err)
		}
		parsedTime, _ := time.Parse(time.RFC3339, createdAt)

		agent := &Agent{
			ID:        id,
			Name:      name,
			Type:      AgentType(agentType),
			PublicKey: pubKey,
			CreatedAt: parsedTime,
			Metadata:  meta,
		}

		if revokedAt.Valid && revokedAt.String != "" {
			t, _ := time.Parse(time.RFC3339, revokedAt.String)
			agent.RevokedAt = &t
		}

		agents = append(agents, agent)
	}

	return agents, nil
}

// Revoke marks an agent as revoked
func (s *AgentStore) Revoke(id string) error {
	result, err := s.db.Exec(
		`UPDATE agents SET revoked_at = ? WHERE id = ? AND revoked_at IS NULL`,
		time.Now().UTC().Format(time.RFC3339), id,
	)
	if err != nil {
		return fmt.Errorf("failed to revoke agent: %w", err)
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return fmt.Errorf("failed to check rows affected: %w", err)
	}
	if rows == 0 {
		return fmt.Errorf("agent not found or already revoked")
	}
	return nil
}

// generateAgentID is now handled by the exported GenerateAgentID

// ParseAgentID parses an agent ID and returns the prefix
func ParseAgentID(id string) (prefix string, hash string, valid bool) {
	if len(id) < 4 || id[:4] != "aid:" {
		return "", "", false
	}
	return "aid:", id[4:], true
}

// AgentIDInfo contains parsed agent ID information
type AgentIDInfo struct {
	Prefix string
	Hash   string
	FullID string
}

// ParseAgentIDFull parses an agent ID into its components
func ParseAgentIDFull(id string) *AgentIDInfo {
	if len(id) < 4 || id[:4] != "aid:" {
		return nil
	}
	return &AgentIDInfo{
		Prefix: "aid:",
		Hash:   id[4:],
		FullID: id,
	}
}

// ValidateAgentID checks if an agent ID is valid
func ValidateAgentID(id string) bool {
	if len(id) < 4 || id[:4] != "aid:" {
		return false
	}
	// Hash should be 8 bytes in hex (16 characters)
	if len(id[4:]) < 8 {
		return false
	}
	return true
}

// AgentInfo represents agent data for display/output
type AgentInfo struct {
	ID        string    `json:"id"`
	Name      string    `json:"name"`
	Type      string    `json:"type"`
	CreatedAt time.Time `json:"createdAt"`
	Revoked   bool      `json:"revoked"`
}

// ToDisplayInfo converts an Agent to AgentInfo for display
func (a *Agent) ToDisplayInfo() *AgentInfo {
	return &AgentInfo{
		ID:        a.ID,
		Name:      a.Name,
		Type:      string(a.Type),
		CreatedAt: a.CreatedAt,
		Revoked:   a.IsRevoked(),
	}
}

// PrettyPrint prints an agent in a human-readable format
func (a *Agent) PrettyPrint() string {
	revoked := "No"
	if a.IsRevoked() {
		revoked = "Yes"
	}

	return fmt.Sprintf(`Agent ID:      %s
Name:          %s
Type:          %s
Created:       %s
Revoked:       %s
Public Key:    %s...
`,
		a.ID,
		a.Name,
		a.Type,
		a.CreatedAt.Format(time.RFC3339),
		revoked,
		a.PublicKey[:16],
	)
}

// MarshalJSON implements JSON marshaling for Agent
func (a Agent) MarshalJSON() ([]byte, error) {
	type Alias Agent
	return json.Marshal(&struct {
		*Alias
	}{
		Alias: (*Alias)(&a),
	})
}

// Base64Decode decodes a base64 encoded string
func Base64Decode(s string) ([]byte, error) {
	return base64.StdEncoding.DecodeString(s)
}

// Base64Encode encodes bytes to base64
func Base64Encode(data []byte) string {
	return base64.StdEncoding.EncodeToString(data)
}
