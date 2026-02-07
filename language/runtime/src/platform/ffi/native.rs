#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::RuntimeError;
use crate::platform::bindings::native_call;
use crate::platform::ffi::bindings_generated as bindings;
use crate::platform::{
    PlatformError,
    NativeSlice,
    NativeArray,
    RuntimeStatus,
    NativeStringRef,
};

use crate::platform::{fs, resource};
use crate::platform::ffi::{FfiPointer};

/// Stub for destack.ffi.address.
#[unsafe(export_name = "destack.ffi.address")]
pub unsafe extern "C" fn destack_ffi_address(out: *mut u64, pointer: FfiPointer) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(ADDRESS)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, pointer);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.address",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.call.
#[unsafe(export_name = "destack.ffi.call")]
pub unsafe extern "C" fn destack_ffi_call(out: *mut NativeSlice<u8>, symbol: resource::SymbolHandle, abi: u32, flags: u32, arguments: NativeSlice<u8>, resultsize: u32) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(CALL)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, symbol, abi, flags, arguments, resultsize);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.call",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.fromAddress.
#[unsafe(export_name = "destack.ffi.fromAddress")]
pub unsafe extern "C" fn destack_ffi_from_address(out: *mut FfiPointer, address: u64) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(FROM_ADDRESS)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, address);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.fromAddress",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.libraryClose.
#[unsafe(export_name = "destack.ffi.libraryClose")]
pub unsafe extern "C" fn destack_ffi_library_close(handle: resource::LibraryHandle) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(LIBRARY_CLOSE)?;
        let _ = handle;

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.libraryClose",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.libraryOpen.
#[unsafe(export_name = "destack.ffi.libraryOpen")]
pub unsafe extern "C" fn destack_ffi_library_open(out: *mut resource::LibraryHandle, path: fs::OsPath, flags: u32) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(LIBRARY_OPEN)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, path, flags);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.libraryOpen",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.symbolAddress.
#[unsafe(export_name = "destack.ffi.symbolAddress")]
pub unsafe extern "C" fn destack_ffi_symbol_address(out: *mut u64, symbol: resource::SymbolHandle) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(SYMBOL_ADDRESS)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, symbol);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.symbolAddress",
        ))
        .boxed())
    })
}

/// Stub for destack.ffi.symbolLookup.
#[unsafe(export_name = "destack.ffi.symbolLookup")]
pub unsafe extern "C" fn destack_ffi_symbol_lookup(out: *mut resource::SymbolHandle, library: resource::LibraryHandle, name: NativeStringRef) -> RuntimeStatus {
    native_call(|context| {
        context.check_policy(SYMBOL_LOOKUP)?;
        if out.is_null() {
            return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
        }
        let _ = (out, library, name);

        Err(RuntimeError::from(PlatformError::not_supported(
            "destack.ffi.symbolLookup",
        ))
        .boxed())
    })
}

