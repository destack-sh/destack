use crate::diagnostic::{RuntimeError, RuntimeResult};
use crate::platform::PlatformError;
use crate::platform::error::PlatformErrorVm;
use crate::runtime::RuntimeCallContext;
use destack_vm as vm;

/// Stub for destack.error.takePlatformError.
pub(super) fn destack_error_take_platform_error(
    _runtime: &RuntimeCallContext,
    _context: &mut vm::RuntimeContext<'_>,
    errorid: u64,
) -> RuntimeResult<PlatformErrorVm> {
    let _ = errorid;
    Err(RuntimeError::platform(PlatformError::not_supported(
        "destack.error.takePlatformError is not available in the VM yet",
    ))
    .boxed())
}
