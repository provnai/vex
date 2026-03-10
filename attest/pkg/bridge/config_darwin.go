//go:build darwin && cgo
// +build darwin,cgo

package bridge

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../target/debug -L${SRCDIR}/../../../target/debug -lattest_rs -framework Security -framework CoreFoundation
*/
import "C"
