use destack_vm as vm;
use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::{VmSlice, VmArray};
use crate::platform::{fs, resource};
use crate::platform::ffi::{FfiPointer};
use crate::runtime::RuntimeCallContext;

/// Stub for destack.ffi.address.
pub(super) fn destack_ffi_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    pointer: FfiPointer,
) -> RuntimeResult<u64> {
    let _ = pointer;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.address is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.call.
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
        "destack.ffi.call is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.fromAddress.
pub(super) fn destack_ffi_from_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    address: u64,
) -> RuntimeResult<FfiPointer> {
    let _ = address;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.fromAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.libraryClose.
pub(super) fn destack_ffi_library_close(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    handle: resource::LibraryHandle,
) -> RuntimeResult<()> {
    let _ = handle;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.libraryClose is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.libraryOpen.
pub(super) fn destack_ffi_library_open(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    path: fs::OsPathVm,
    flags: u32,
) -> RuntimeResult<resource::LibraryHandle> {
    let _ = (path, flags);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.libraryOpen is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.symbolAddress.
pub(super) fn destack_ffi_symbol_address(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    symbol: resource::SymbolHandle,
) -> RuntimeResult<u64> {
    let _ = symbol;
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.symbolAddress is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.ffi.symbolLookup.
pub(super) fn destack_ffi_symbol_lookup(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    library: resource::LibraryHandle,
    name: vm::StringHandle,
) -> RuntimeResult<resource::SymbolHandle> {
    let _ = (library, name);
    Err(RuntimeError::from(PlatformError::not_supported(
        "destack.ffi.symbolLookup is not available in the VM yet",
    ))
    .boxed())
}

