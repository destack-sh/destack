use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::{PlatformError, VmSlice};
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.random.fillBytes.
pub(super) fn destack_random_fill_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.random.fillBytes is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.nextU64.
pub(super) fn destack_random_next_u64(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
) -> RuntimeResult<u64> {
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.random.nextU64 is not available in the VM yet",
    ))
    .boxed())
}

/// Stub for destack.random.secureBytes.
pub(super) fn destack_random_secure_bytes(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    buffer: VmSlice<u8>,
) -> RuntimeResult<()> {
    let _ = buffer;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.random.secureBytes is not available in the VM yet",
    ))
    .boxed())
}
