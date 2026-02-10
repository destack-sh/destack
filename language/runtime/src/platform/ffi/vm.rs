use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::ffi::FfiPointer;
use crate::platform::{PlatformError, VmSlice, fs, resource};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.ffi.call.call.
pub(super) fn destack_ffi_call(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    symbol: resource::SymbolHandle,
    abi: u32,
    flags: u32,
    arguments: VmSlice<u8>,
    resultsize: u32,
) -> RuntimeResult<VmSlice<u8>> {
    let _ = (symbol, abi, flags, arguments, resultsize);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.call.call is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.library.close.
pub(super) fn destack_ffi_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::LibraryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.library.close is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.library.open.
pub(super) fn destack_ffi_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
    flags: u32,
) -> RuntimeResult<resource::LibraryHandle> {
    let _ = (path, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.library.open is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.pointer.address.
pub(super) fn destack_ffi_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pointer: FfiPointer,
) -> RuntimeResult<u64> {
    let _ = pointer;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.pointer.address is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.pointer.fromAddress.
pub(super) fn destack_ffi_from_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: u64,
) -> RuntimeResult<FfiPointer> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.pointer.fromAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.symbol.address.
pub(super) fn destack_ffi_symbol_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    symbol: resource::SymbolHandle,
) -> RuntimeResult<u64> {
    let _ = symbol;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.symbol.address is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.symbol.lookup.
pub(super) fn destack_ffi_symbol_lookup(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    library: resource::LibraryHandle,
    name: vm::StringHandle,
) -> RuntimeResult<resource::SymbolHandle> {
    let _ = (library, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.symbol.lookup is not available in the VM yet",
    ))
    .boxed())
}
