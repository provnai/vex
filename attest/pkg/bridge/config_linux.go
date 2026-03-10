//go:build linux && cgo
// +build linux,cgo

package bridge

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../target/debug -L${SRCDIR}/../../../target/debug -lattest_rs -lssl -lcrypto -ltss2-esys -ltss2-mu -ltss2-tctildr -lm -ldl -lpthread
*/
import "C"
