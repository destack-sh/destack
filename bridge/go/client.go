package destack

/*
#cgo linux LDFLAGS: -ldl
#include <stdbool.h>
#include <stdint.h>
#include <stdlib.h>

#if defined(_WIN32)
#include <windows.h>
#else
#include <dlfcn.h>
#endif

typedef uint32_t (*destack_capi_abi_version_fn)(void);
typedef const char *(*destack_capi_version_fn)(void);
typedef bool (*destack_capi_is_available_fn)(void);

static void *destack_capi_open(const char *path) {
#if defined(_WIN32)
	return (void *)LoadLibraryA(path);
#else
	return dlopen(path, RTLD_NOW | RTLD_LOCAL);
#endif
}

static void *destack_capi_symbol(void *handle, const char *name) {
#if defined(_WIN32)
	return (void *)GetProcAddress((HMODULE)handle, name);
#else
	return dlsym(handle, name);
#endif
}

static uint32_t destack_capi_call_abi_version(void *fn_ptr) {
	return ((destack_capi_abi_version_fn)fn_ptr)();
}

static const char *destack_capi_call_version(void *fn_ptr) {
	return ((destack_capi_version_fn)fn_ptr)();
}

static bool destack_capi_call_is_available(void *fn_ptr) {
	return ((destack_capi_is_available_fn)fn_ptr)();
}
*/
import "C"

import (
	"os"
	"runtime"
	"sync"
	"unsafe"
)

const (
	// Backend is the static backend marker for this package.
	Backend = "go"
	// Version is the package version fallback.
	Version = "0.55.4"
)

type capiState struct {
	abiVersionFn  unsafe.Pointer
	versionFn     unsafe.Pointer
	isAvailableFn unsafe.Pointer
}

var (
	capiOnce sync.Once
	capi     *capiState
)

// Client is a client type for Destack.
type Client struct{}

// NewClient creates a new client.
func NewClient() Client {
	return Client{}
}

// BackendName returns the backend marker.
func (client Client) BackendName() string {
	return Backend
}

// CapiAbiVersion returns the capi abi version when available.
func (client Client) CapiAbiVersion() uint32 {
	state := loadCapi()
	if state == nil {
		return 0
	}

	return uint32(C.destack_capi_call_abi_version(state.abiVersionFn))
}

// CapiIsAvailable returns the capi runtime availability.
func (client Client) CapiIsAvailable() bool {
	state := loadCapi()
	if state == nil {
		return false
	}

	return bool(C.destack_capi_call_is_available(state.isAvailableFn))
}

// VersionString returns the package version.
func (client Client) VersionString() string {
	state := loadCapi()
	if state == nil {
		return Version
	}

	value := C.destack_capi_call_version(state.versionFn)
	if value == nil {
		return Version
	}

	return C.GoString(value)
}

// resolve and cache capi function pointers once
func loadCapi() *capiState {
	capiOnce.Do(func() {
		libraryPath := os.Getenv("DESTACK_CAPI_LIB")
		if libraryPath == "" {
			libraryPath = defaultCapiLibraryName()
		}

		pathCString := C.CString(libraryPath)
		defer C.free(unsafe.Pointer(pathCString))

		handle := C.destack_capi_open(pathCString)
		if handle == nil {
			return
		}

		abiVersionSymbol := C.CString("destack_capi_abi_version")
		defer C.free(unsafe.Pointer(abiVersionSymbol))
		versionSymbol := C.CString("destack_capi_version")
		defer C.free(unsafe.Pointer(versionSymbol))
		isAvailableSymbol := C.CString("destack_capi_is_available")
		defer C.free(unsafe.Pointer(isAvailableSymbol))

		abiVersionFn := C.destack_capi_symbol(handle, abiVersionSymbol)
		versionFn := C.destack_capi_symbol(handle, versionSymbol)
		isAvailableFn := C.destack_capi_symbol(handle, isAvailableSymbol)
		if abiVersionFn == nil || versionFn == nil || isAvailableFn == nil {
			return
		}

		capi = &capiState{
			abiVersionFn:  abiVersionFn,
			versionFn:     versionFn,
			isAvailableFn: isAvailableFn,
		}
	})

	return capi
}

// pick the default shared library name by host os
func defaultCapiLibraryName() string {
	switch runtime.GOOS {
	case "darwin":
		return "libdestack_capi.dylib"
	case "windows":
		return "destack_capi.dll"
	default:
		return "libdestack_capi.so"
	}
}
