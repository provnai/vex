//go:build !cgo
// +build !cgo

package bridge

func IsCgoEnabled() bool {
	return false
}
