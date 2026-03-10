//go:build cgo
// +build cgo

package bridge

/*
#include <stdlib.h>
#include <stdbool.h>

#cgo LDFLAGS: -L../../attest-rs/target/debug -L../../attest-rs/target/release -lattest_rs -lm -ldl -lpthread

void* attest_agent_new();
void attest_agent_free(void* ptr);
void attest_free_string(char* ptr);
char* attest_agent_get_id(void* ptr);
unsigned char* attest_seal(const unsigned char* data, size_t data_len, size_t* out_len);
unsigned char* attest_unseal(const unsigned char* blob, size_t blob_len, size_t* out_len);
void attest_free_buffer(unsigned char* ptr, size_t len);
void attest_set_strict_hardware(bool strict);
void* attest_policy_engine_new();
void attest_policy_engine_free(void* ptr);
void attest_policy_engine_load_defaults(void* ptr);
bool attest_verify_intent(void* agent_ptr, void* policy_ptr, const char* intent_json);
*/
import "C"

import (
	"fmt"
	"unsafe"
)

// SetStrictHardware enables or disables software fallback for TPM operations
func SetStrictHardware(strict bool) {
	C.attest_set_strict_hardware(C.bool(strict))
}

type AttestAgent struct {
	ptr unsafe.Pointer
}

// NewAttestAgent creates a new identity using the Rust core
func NewAttestAgent() *AttestAgent {
	return &AttestAgent{ptr: C.attest_agent_new()}
}

// Free deallocates the Rust-side agent
func (a *AttestAgent) Free() {
	if a.ptr != nil {
		C.attest_agent_free(a.ptr)
		a.ptr = nil
	}
}

// GetID returns the cryptographic identifier of the agent
func (a *AttestAgent) GetID() string {
	if a.ptr == nil {
		return ""
	}
	cStr := C.attest_agent_get_id(a.ptr)
	defer C.attest_free_string(cStr)
	return C.GoString(cStr)
}

// Seal uses the TPM to securely wrap data
func Seal(data []byte) ([]byte, error) {
	if len(data) == 0 {
		return nil, fmt.Errorf("cannot seal empty data")
	}

	var outLen C.size_t
	ptr := C.attest_seal((*C.uchar)(unsafe.Pointer(&data[0])), C.size_t(len(data)), &outLen)
	if ptr == nil {
		return nil, fmt.Errorf("TPM seal failed in Rust core")
	}

	result := C.GoBytes(unsafe.Pointer(ptr), C.int(outLen))
	C.attest_free_buffer(ptr, outLen)
	return result, nil
}

// Unseal unwraps TPM-sealed data
func Unseal(blob []byte) ([]byte, error) {
	if len(blob) == 0 {
		return nil, fmt.Errorf("cannot unseal empty blob")
	}

	var outLen C.size_t
	ptr := C.attest_unseal((*C.uchar)(unsafe.Pointer(&blob[0])), C.size_t(len(blob)), &outLen)
	if ptr == nil {
		return nil, fmt.Errorf("TPM unseal failed in Rust core")
	}

	result := C.GoBytes(unsafe.Pointer(ptr), C.int(outLen))
	C.attest_free_buffer(ptr, outLen)
	return result, nil
}

// PolicyEngine manages security guardrails
type PolicyEngine struct {
	ptr unsafe.Pointer
}

// NewPolicyEngine creates a new policy engine using the Rust core
func NewPolicyEngine() *PolicyEngine {
	return &PolicyEngine{ptr: C.attest_policy_engine_new()}
}

// Free deallocates the Rust-side engine
func (e *PolicyEngine) Free() {
	if e.ptr != nil {
		C.attest_policy_engine_free(e.ptr)
		e.ptr = nil
	}
}

// LoadDefaults loads built-in safety policies
func (e *PolicyEngine) LoadDefaults() {
	if e.ptr != nil {
		C.attest_policy_engine_load_defaults(e.ptr)
	}
}

// VerifyIntent checks if an intent goal is allowed by the policy engine
func (a *AttestAgent) VerifyIntent(engine *PolicyEngine, intentJSON string) bool {
	if a.ptr == nil || engine == nil || engine.ptr == nil {
		return false
	}
	cStr := C.CString(intentJSON)
	defer C.free(unsafe.Pointer(cStr))
	return bool(C.attest_verify_intent(a.ptr, engine.ptr, cStr))
}
