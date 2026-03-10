//go:build windows && cgo
// +build windows,cgo

package bridge

/*
#cgo LDFLAGS: -Wl,-rpath,${SRCDIR}/../../../target/x86_64-pc-windows-gnu/release -L${SRCDIR}/../../../target/x86_64-pc-windows-gnu/release -lattest_rs -lbcrypt -lws2_32 -luserenv -lntdll -lole32
*/
import "C"
