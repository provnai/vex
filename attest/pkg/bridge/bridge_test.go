package bridge

import (
	"bytes"
	"testing"
)

func TestBridgeIdentity(t *testing.T) {
	if !IsCgoEnabled() {
		t.Skip("CGO disabled: skipping Rust bridge identity test")
	}
	agent := NewAttestAgent()
	defer agent.Free()

	id := agent.GetID()
	if id == "" {
		t.Fatal("Failed to get agent ID from Rust core")
	}
	t.Logf("Agent ID: %s", id)

	if len(id) < 10 {
		t.Errorf("ID seems too short: %s", id)
	}
}

func TestBridgeTPM(t *testing.T) {
	data := []byte("high-entropy-seed-data")

	sealed, err := Seal(data)
	if err != nil {
		t.Skipf("TPM Seal skipped (likely no hardware): %v", err)
		return
	}

	unsealed, err := Unseal(sealed)
	if err != nil {
		t.Fatalf("TPM Unseal failed: %v", err)
	}

	if !bytes.Equal(data, unsealed) {
		t.Errorf("Unsealed data does not match original: expected %v, got %v", data, unsealed)
	}
}
