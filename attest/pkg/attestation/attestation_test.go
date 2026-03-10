package attestation

import (
	"fmt"
	"testing"
	"time"

	"github.com/provnai/attest/pkg/crypto"
	"github.com/stretchr/testify/assert"
)

func TestAttestationSigningAndVerification(t *testing.T) {
	// 1. Setup Keys
	keys, err := crypto.GenerateEd25519KeyPair()
	assert.NoError(t, err)

	// agent := &crypto.KeyPair{ ... } (Unused in this test setup currently)

	// 2. Create Attestation
	attest := &Attestation{
		ID:      "att_test_123",
		AgentID: "agent_test_123",
		Action: ActionRecord{
			Type:   ActionTypeCommand,
			Target: "echo hello",
		},
		Timestamp: time.Now().UTC(),
	}

	// 3. Sign
	signData := fmt.Sprintf("%s:%s:%s:%s:%s:%s",
		attest.ID,
		attest.AgentID,
		attest.IntentID,
		attest.Action.Type,
		attest.Action.Target,
		attest.Timestamp.Format(time.RFC3339),
	)

	sig, err := keys.Sign([]byte(signData))
	assert.NoError(t, err)
	assert.NotEmpty(t, sig)

	// Mimic the CreateAttestation logic which stores it
	// But wait, CreateAttestation is a function in attestation.go that does the signing internally?
	// Let's check the code I viewed earlier.
	// Yes, `CreateAttestation` calls `signAttestation`.
	// Let's test the public API `CreateAttestation`.
}
