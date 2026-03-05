#![allow(dead_code)]
#![allow(unused_imports)]
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ffi::FfiPointer;
use crate::platform::{PlatformError, VmArray, VmSlice, fs, resource};
use crate::runtime::BindingCallContext;
use destack_vm as vm;

/// Call one foreign symbol using raw ABI argument and result buffers.
///
/// Invoke one host symbol trampoline with caller-provided ABI payload bytes and explicit ABI selectors.
/// ABI packing, alignment, and calling convention semantics are runtime-defined and backend-specific.
///
/// # Platform
/// Unix and Windows.
/// Uses host ABI trampolines over process calling conventions.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.call`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_ffi_call(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    symbol: resource::SymbolHandle,
    abi: u32,
    flags: u32,
    arguments: VmSlice<u8>,
    resultsize: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (symbol, abi, flags, arguments, resultsize);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.call.invoke")).boxed())
}

/// Close a dynamic library.
///
/// Release one loaded dynamic library handle.
/// Unload timing and symbol invalidation follow host loader semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses dlclose on Unix and FreeLibrary on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.load`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_ffi_close(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    handle: resource::LibraryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.library.close")).boxed())
}

/// Open a dynamic library.
///
/// Load one host dynamic library and return a stable runtime handle.
/// Symbol visibility and loader flags follow host dynamic loader semantics.
///
/// # Platform
/// Unix and Windows.
/// Uses dlopen on Unix and LoadLibraryExW on Windows.
///
/// # Errors
/// Returns ioNotFound, ioPermissionDenied, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.load`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_ffi_open(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    path: fs::OsPathVm,
    flags: u32,
) -> RuntimeResult<resource::LibraryHandle> {
    let _ = (path, flags);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.library.open")).boxed())
}

/// Return the raw address for an opaque pointer.
///
/// Unwrap one foreign pointer into a raw machine address.
/// Address interpretation depends on caller ABI and target architecture.
///
/// # Platform
/// Unix and Windows.
/// Uses host-process pointer wrapper logic.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.pointer`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_ffi_address(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    pointer: FfiPointer,
) -> RuntimeResult<u64> {
    let _ = pointer;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.pointer.address")).boxed())
}

/// Create a pointer from a raw address.
///
/// Wrap one raw machine address as an opaque foreign pointer value.
/// Pointer validity and lifetime are controlled by caller and host ABI contracts.
///
/// # Platform
/// Unix and Windows.
/// Uses host-process pointer wrapper logic.
///
/// # Errors
/// Returns invalidArgument, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.pointer`.
///
/// # Replay
/// Deterministic.
pub(crate) fn destack_ffi_from_address(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    address: u64,
) -> RuntimeResult<FfiPointer> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.pointer.fromAddress",
    ))
    .boxed())
}

/// Return a raw address for a resolved symbol handle.
///
/// Return one raw process address for a previously resolved symbol.
/// Address lifetime is tied to the owning dynamic library handle.
///
/// # Platform
/// Unix and Windows.
/// Uses runtime symbol table state after host resolution.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.symbol`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_ffi_symbol_address(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    symbol: resource::SymbolHandle,
) -> RuntimeResult<u64> {
    let _ = symbol;
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.symbol.address")).boxed())
}

/// Resolve a symbol from a loaded library.
///
/// Resolve one symbol name in one loaded library and return a stable symbol handle.
/// Symbol lookup rules follow host dynamic loader name resolution behavior.
///
/// # Platform
/// Unix and Windows.
/// Uses dlsym on Unix and GetProcAddress on Windows.
///
/// # Errors
/// Returns invalidArgument, ioNotFound, ioInvalidData, notSupported.
///
/// # Security
/// Requires `ffi.symbol`.
///
/// # Replay
/// External, nonrecordable.
pub(crate) fn destack_ffi_symbol_lookup(
    _binding: &BindingCallContext,
    _context: &mut vm::ExternalCallContext<'_>,
    library: resource::LibraryHandle,
    name: vm::StringHandle,
) -> RuntimeResult<resource::SymbolHandle> {
    let _ = (library, name);
    Err(RuntimeError::from(PlatformError::not_supported("destack.ffi.symbol.lookup")).boxed())
}
