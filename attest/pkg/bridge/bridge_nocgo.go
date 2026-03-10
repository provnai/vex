//go:build !cgo
// +build !cgo

package bridge

import "fmt"

func SetStrictHardware(strict bool) {}

type AttestAgent struct{}

func (a *AttestAgent) Free() {}

func (a *AttestAgent) GetID() string { return "" }

func NewAttestAgent() *AttestAgent { return &AttestAgent{} }

func Seal(data []byte) ([]byte, error) {
	return nil, fmt.Errorf("hardware seal requires CGO and the Rust security core")
}

func Unseal(blob []byte) ([]byte, error) {
	return nil, fmt.Errorf("hardware unseal requires CGO and the Rust security core")
}
