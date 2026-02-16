use destack_vm as vm;

use crate::diagnostic::RuntimeResult;
use crate::platform::ffi::{FfiPointer, vm as ffi_vm};
use crate::platform::{VmSlice, resource};
use crate::runtime::RuntimeCallContext;

/// Return the raw address for an opaque pointer.
///
/// Unwrap one foreign pointer into a raw machine address.
/// Address interpretation depends on caller ABI and target architecture.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime pointer wrapper logic only.
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
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    pointer: FfiPointer,
) -> RuntimeResult<u64> {
    ffi_vm::destack_ffi_address(runtime, context, pointer)
}

/// Call one foreign symbol using raw ABI argument and result buffers.
///
/// Invoke one host symbol trampoline with caller-provided ABI payload bytes and explicit ABI selectors.
/// ABI packing, alignment, and calling convention semantics are runtime-defined and backend-specific.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime ABI trampolines over host process calling conventions.
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
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    symbol: resource::SymbolHandle,
    abi: u32,
    flags: u32,
    arguments: VmSlice<u8>,
    resultsize: u32,
) -> RuntimeResult<VmSlice<u8>> {
    ffi_vm::destack_ffi_call(runtime, context, symbol, abi, flags, arguments, resultsize)
}

/// Create a pointer from a raw address.
///
/// Wrap one raw machine address as an opaque foreign pointer value.
/// Pointer validity and lifetime are controlled by caller and host ABI contracts.
///
/// # Platform
/// Runtime-managed on all targets.
/// Uses runtime pointer wrapper logic only.
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
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    address: u64,
) -> RuntimeResult<FfiPointer> {
    ffi_vm::destack_ffi_from_address(runtime, context, address)
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
    runtime: &RuntimeCallContext,
    context: &mut vm::ExternalCallContext<'_>,
    symbol: resource::SymbolHandle,
) -> RuntimeResult<u64> {
    ffi_vm::destack_ffi_symbol_address(runtime, context, symbol)
}
