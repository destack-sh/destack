#![allow(clippy::missing_safety_doc)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ffi::bindings_generated as bindings;
use crate::platform::{NativeSlice, NativeStringRef, PlatformError};

use crate::runtime::RuntimeCallContext;
use bindings::*;

use crate::platform::ffi::FfiPointer;
use crate::platform::{fs, resource};

/// Stub for destack.ffi.call.call.
pub unsafe fn destack_ffi_call(
    context: &RuntimeCallContext,
    out: *mut NativeSlice<u8>,
    symbol: resource::SymbolHandle,
    abi: u32,
    flags: u32,
    arguments: NativeSlice<u8>,
    resultsize: u32,
) -> RuntimeResult<()> {
    context.check_policy(FFI_CALL_CALL)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, symbol, abi, flags, arguments, resultsize);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.call.call")).boxed())
}

/// Stub for destack.ffi.library.close.
pub unsafe fn destack_ffi_close(
    context: &RuntimeCallContext,
    handle: resource::LibraryHandle,
) -> RuntimeResult<()> {
    context.check_policy(FFI_LIBRARY_CLOSE)?;
    let _ = handle;

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.library.close")).boxed())
}

/// Stub for destack.ffi.library.open.
pub unsafe fn destack_ffi_open(
    context: &RuntimeCallContext,
    out: *mut resource::LibraryHandle,
    path: fs::OsPath,
    flags: u32,
) -> RuntimeResult<()> {
    context.check_policy(FFI_LIBRARY_OPEN)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, path, flags);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.library.open")).boxed())
}

/// Stub for destack.ffi.pointer.address.
pub unsafe fn destack_ffi_address(
    context: &RuntimeCallContext,
    out: *mut u64,
    pointer: FfiPointer,
) -> RuntimeResult<()> {
    context.check_policy(FFI_POINTER_ADDRESS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, pointer);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.pointer.address")).boxed())
}

/// Stub for destack.ffi.pointer.fromAddress.
pub unsafe fn destack_ffi_from_address(
    context: &RuntimeCallContext,
    out: *mut FfiPointer,
    address: u64,
) -> RuntimeResult<()> {
    context.check_policy(FFI_POINTER_FROM_ADDRESS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, address);

    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.pointer.fromAddress",
    ))
    .boxed())
}

/// Stub for destack.ffi.symbol.address.
pub unsafe fn destack_ffi_symbol_address(
    context: &RuntimeCallContext,
    out: *mut u64,
    symbol: resource::SymbolHandle,
) -> RuntimeResult<()> {
    context.check_policy(FFI_SYMBOL_ADDRESS)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, symbol);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.symbol.address")).boxed())
}

/// Stub for destack.ffi.symbol.lookup.
pub unsafe fn destack_ffi_symbol_lookup(
    context: &RuntimeCallContext,
    out: *mut resource::SymbolHandle,
    library: resource::LibraryHandle,
    name: NativeStringRef,
) -> RuntimeResult<()> {
    context.check_policy(FFI_SYMBOL_LOOKUP)?;
    if out.is_null() {
        return Err(RuntimeError::from(PlatformError::null_pointer("out")).boxed());
    }
    let _ = (out, library, name);

    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.symbol.lookup")).boxed())
}
