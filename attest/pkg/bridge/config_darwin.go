//go:build darwin && cgo
// +build darwin,cgo

package bridge

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR}/../../attest-rs/target/debug -L${SRCDIR}/../../attest-rs/target/debug -lattest_rs -framework Security -framework CoreFoundation
*/
import "C"
